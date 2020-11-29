use super::*;

use slab::Slab;
use crate::terminal::NonBlockingStdin;

lazy_static! {
    static ref GLOBAL_WATCHER_SET: Mutex<Option<GlobalWatcherSet>> = Mutex::new(None);
}

task_local! {
    // TODO: this doesn't need to be a Mutex
    static EVENT_MAP: Mutex<Vec<&'static (dyn Fn(Event) -> Option<Event> + Sync + Send)>>;
}

pub(crate) async fn with_input_handling<F: Future>(stdin: NonBlockingStdin, future: F)
    -> io::Result<F::Output>
{
    EVENT_MAP.scope(Mutex::new(Vec::new()), async {
        let event_task = EventTask::new(stdin);
        let mut global_set_opt = GLOBAL_WATCHER_SET.lock().unwrap();
        *global_set_opt = Some(GlobalWatcherSet {
            watcher_wakers: Slab::new(),
            num_polled_this_round: 0,
            odd_numbered_round: false,
            current_event: Poll::Pending,
            event_task_waker_opt: None,
        });
        drop(global_set_opt);

        let join_handle = tokio::spawn(event_task);
        let ret = future.await;
        let mut global_set_opt = GLOBAL_WATCHER_SET.lock().unwrap();
        {
            let global_set = global_set_opt.as_mut().unwrap();
            if let Some(event_task_waker) = global_set.event_task_waker_opt.as_ref() {
                event_task_waker.wake_by_ref();
            }
        }
        *global_set_opt = None;
        drop(global_set_opt);

        join_handle.await.unwrap()?;
        Ok(ret)
    }).await
}

pub(crate) fn with_event_map<M, F, R>(map: M, func: F) -> R
where
    F: FnOnce() -> R,
    F: panic::UnwindSafe,
    M: Fn(Event) -> Option<Event> + Sync + Send,
{
    let map: &(dyn Fn(Event) -> Option<Event> + Sync + Send) = &map;
    let map: &'static (dyn Fn(Event) -> Option<Event> + Sync + Send) = unsafe {
        mem::transmute(map)
    };
    let event_map_len = EVENT_MAP.with(|event_map| {
        let mut event_map = event_map.lock().unwrap();
        let event_map_len = event_map.len();
        event_map.push(map);
        event_map_len
    });
    let ret_res = panic::catch_unwind(func);
    EVENT_MAP.with(|event_map| {
        let mut event_map = event_map.lock().unwrap();
        event_map.pop();
        assert_eq!(event_map.len(), event_map_len);
    });
    match ret_res {
        Ok(ret) => ret,
        Err(err) => panic::resume_unwind(err),
    }
}

struct GlobalWatcherSet {
    watcher_wakers: Slab<Option<Waker>>,
    num_polled_this_round: usize,
    odd_numbered_round: bool,
    current_event: Poll<Option<Event>>,
    event_task_waker_opt: Option<Waker>,
}

#[pin_project]
struct EventTask {
    #[pin]
    events: Events,
}

pub struct EventWatcher {
    odd_numbered_round: bool,
    key: usize,
}

impl EventWatcher {
    pub fn new() -> Option<EventWatcher> {
        let mut global_set_opt = GLOBAL_WATCHER_SET.lock().unwrap();
        let global_set = global_set_opt.as_mut()?;
        let key = global_set.watcher_wakers.insert(None);
        global_set.num_polled_this_round += 1;
        let odd_numbered_round = !global_set.odd_numbered_round;
        trace!("Creating EventWatcher. Including us, {} of {} watchers have polled", global_set.num_polled_this_round, global_set.watcher_wakers.len());
        Some(EventWatcher { odd_numbered_round, key })
    }
}

impl Drop for EventWatcher {
    fn drop(&mut self) {
        let mut global_set_opt = GLOBAL_WATCHER_SET.lock().unwrap();
        if let Some(global_set) = global_set_opt.as_mut() {
            let _watcher_opt = global_set.watcher_wakers.remove(self.key);

            trace!("dropping EventWatcher");
            if self.odd_numbered_round != global_set.odd_numbered_round {
                trace!("we had polled. decrementing num_polled_this_round");
                global_set.num_polled_this_round -= 1;
            } else if global_set.num_polled_this_round == global_set.watcher_wakers.len() {
                trace!("we had not polled, all remaining watchers have now polled, waking event task");
                global_set.current_event = Poll::Pending;
                if let Some(event_task_waker) = global_set.event_task_waker_opt.as_ref() {
                    event_task_waker.wake_by_ref();
                }
            }
            trace!("now that we're gone, {} of {} remaining watchers have polled", global_set.num_polled_this_round, global_set.watcher_wakers.len());
        }
    }
}

impl EventTask {
    pub fn new(stdin: NonBlockingStdin) -> EventTask {
        let events = Events::new(stdin);
        EventTask { events }
    }
}

impl Future for EventTask {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.project();
        trace!("polling EventTask");
        let mut global_set_opt = GLOBAL_WATCHER_SET.lock().unwrap();
        let global_set = match global_set_opt.as_mut() {
            Some(global_set) => global_set,
            None => return Poll::Ready(Ok(())),
        };
        global_set.event_task_waker_opt = Some(cx.waker().clone());

        if global_set.num_polled_this_round < global_set.watcher_wakers.len() {
            trace!("not everyone has polled yet. sleeping");
            return Poll::Pending;
        }

        trace!("ready to poll the input again");
        let err_opt = match this.events.poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                global_set.current_event = Poll::Ready(Some(event));
                None
            },
            Poll::Ready(None) => {
                global_set.current_event = Poll::Ready(None);
                None
            },
            Poll::Ready(Some(Err(e))) => {
                global_set.current_event = Poll::Ready(None);
                Some(e)
            },
            Poll::Pending => {
                // TODO: is this correct?
                // This line wasn't here before, just adding it now while refactoring coz it seems
                // necessary.
                global_set.current_event = Poll::Pending;

                trace!("input not ready");
                return Poll::Pending;
            },
        };
        trace!("new input ready. waking everybody");

        global_set.num_polled_this_round = 0;
        global_set.odd_numbered_round ^= true;

        for (_key, watcher_opt) in global_set.watcher_wakers.iter() {
            if let Some(watcher) = watcher_opt.as_ref() {
                watcher.wake_by_ref();
            }
        }

        match err_opt {
            None => Poll::Pending,
            Some(err) => Poll::Ready(Err(err)),
        }
    }
}

impl Stream for EventWatcher {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Option<Event>>
    {
        let this = Pin::into_inner(self);
        trace!("polling EventWatcher");
        let mut global_set_opt = GLOBAL_WATCHER_SET.lock().unwrap();
        let global_set = match global_set_opt.as_mut() {
            Some(global_set) => global_set,
            None => return Poll::Ready(None),
        };
        global_set.watcher_wakers[this.key] = Some(cx.waker().clone());

        if this.odd_numbered_round != global_set.odd_numbered_round {
            trace!("already polled this round");
            return Poll::Pending;
        }

        trace!("we haven't polled yet this round (odd = {})", this.odd_numbered_round);
        global_set.num_polled_this_round += 1;
        this.odd_numbered_round ^= true;

        let ret = match global_set.current_event {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(ref event)) => {
                let mut event = event.clone();
                EVENT_MAP.with(|event_map| {
                    let event_map = event_map.lock().unwrap();
                    let mut event_maps = event_map.iter();
                    loop {
                        event = match event_maps.next() {
                            Some(event_map) => match event_map(event) {
                                Some(event) => event,
                                None => break Poll::Pending,
                            },
                            None => break Poll::Ready(Some(event)),
                        };
                    }
                })
            },
        };

        trace!("got ret == {:?}", ret);
        trace!("{} of {} watchers have polled", global_set.num_polled_this_round, global_set.watcher_wakers.len());
        if global_set.num_polled_this_round == global_set.watcher_wakers.len() {
            trace!("everyone has polled. waking the event task");
            global_set.current_event = Poll::Pending;
            if let Some(event_task_waker) = global_set.event_task_waker_opt.as_ref() {
                event_task_waker.wake_by_ref();
            }
        }

        ret
    }
}

impl FusedStream for EventWatcher {
    fn is_terminated(&self) -> bool {
        GLOBAL_WATCHER_SET.lock().unwrap().is_none()
    }
}


