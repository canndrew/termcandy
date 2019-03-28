use lazy_static::lazy_static;
use futures::{Async, Future, Stream};
use std::sync::Mutex;
use slab::Slab;
use futures::task::{current, Task};
use futures::task_local;
use termion::event::{Key, Event, MouseEvent, MouseButton};
use crate::input::Events;
use std::sync::Arc;
use failure::format_err;
use crate::io::NonBlockingStdin;
use std::{mem, io};
use log::trace;

lazy_static! {
    static ref GLOBAL_WATCHER_SET: Mutex<Option<GlobalWatcherSet>> = Mutex::new(None);
}

task_local!(static EVENT_MAP: Mutex<Vec<&'static (dyn Fn(Event) -> Option<Event> + Sync + Send)>> = Mutex::new(Vec::new()));

pub(crate) fn with_event_map<M, F, R>(map: M, func: F) -> R
where
    F: FnOnce() -> R,
    M: Fn(Event) -> Option<Event> + Sync + Send,
{
    let map: &(dyn Fn(Event) -> Option<Event> + Sync + Send) = &map;
    let map: &'static (dyn Fn(Event) -> Option<Event> + Sync + Send) = unsafe {
        mem::transmute(map)
    };
    EVENT_MAP.with(|event_map| {
        let mut event_map = event_map.lock().unwrap();
        event_map.push(map);
    });
    let ret = func();
    EVENT_MAP.with(|event_map| {
        let mut event_map = event_map.lock().unwrap();
        event_map.pop();
    });
    ret
}

struct GlobalWatcherSet {
    watcher_tasks: Slab<Task>,
    num_polled_this_round: usize,
    odd_numbered_round: bool,
    current_event: Result<Async<Option<Event>>, Arc<failure::Error>>,
    event_task: Task,
}

struct EventTask {
    events: Events,
}

pub(crate) struct EventTaskHandle {
    _priv: (),
}

struct EventWatcher {
    odd_numbered_round: bool,
    key: usize,
}

impl EventWatcher {
    pub fn new() -> EventWatcher {
        let mut global_set = GLOBAL_WATCHER_SET.lock().unwrap();
        let global_set = global_set.as_mut().unwrap();
        let key = global_set.watcher_tasks.insert(current());
        global_set.num_polled_this_round += 1;
        let odd_numbered_round = !global_set.odd_numbered_round;
        trace!("creating EventWatcher, with us {} of {} watchers have polled", global_set.num_polled_this_round, global_set.watcher_tasks.len());
        EventWatcher { odd_numbered_round, key }
    }
}

impl Drop for EventWatcher {
    fn drop(&mut self) {
        let mut global_set = GLOBAL_WATCHER_SET.lock().unwrap();
        let global_set = global_set.as_mut().unwrap();
        let _task = global_set.watcher_tasks.remove(self.key);

        trace!("dropping EventWatcher");
        if self.odd_numbered_round != global_set.odd_numbered_round {
            trace!("we had polled. decrementing num_polled_this_round");
            global_set.num_polled_this_round -= 1;
        } else if global_set.num_polled_this_round == global_set.watcher_tasks.len() {
            trace!("we had not polled, all remaining watchers have now polled, waking event task");
            global_set.current_event = Ok(Async::NotReady);
            global_set.event_task.notify();
        }
        trace!("now that we're gone, {} of {} remaining watchers have polled", global_set.num_polled_this_round, global_set.watcher_tasks.len());
    }
}

impl EventTask {
    pub fn new(stdin: NonBlockingStdin) -> io::Result<EventTask> {
        let events = Events::new(stdin)?;
        Ok(EventTask { events })
    }
}

impl Future for EventTask {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<()>, ()> {
        trace!("polling EventTask");
        let mut global_set = GLOBAL_WATCHER_SET.lock().unwrap();
        let global_set = match global_set.as_mut() {
            Some(global_set) => global_set,
            None => return Ok(Async::Ready(())),
        };
        global_set.event_task = current();

        if global_set.num_polled_this_round < global_set.watcher_tasks.len() {
            trace!("not everyone has polled yet. sleeping");
            return Ok(Async::NotReady);
        }

        trace!("ready to poll the input again");
        let exit = match self.events.poll() {
            Ok(Async::Ready(event_opt)) => {
                global_set.current_event = Ok(Async::Ready(event_opt));
                false
            },
            Ok(Async::NotReady) => {
                trace!("input not ready");
                return Ok(Async::NotReady);
            },
            Err(e) => {
                global_set.current_event = Err(Arc::new(e));
                true
            },
        };
        trace!("new input ready. waking everybody");

        global_set.num_polled_this_round = 0;
        global_set.odd_numbered_round ^= true;

        for (_, watcher_task) in global_set.watcher_tasks.iter() {
            watcher_task.notify();
        }

        if exit {
            Err(())
        } else {
            Ok(Async::NotReady)
        }
    }
}

impl EventTaskHandle {
    pub fn new(stdin: NonBlockingStdin) -> io::Result<EventTaskHandle> {
        let event_task = EventTask::new(stdin)?;
        let mut global_set = GLOBAL_WATCHER_SET.lock().unwrap();
        *global_set = Some(GlobalWatcherSet {
            watcher_tasks: Slab::new(),
            num_polled_this_round: 0,
            odd_numbered_round: false,
            current_event: Ok(Async::NotReady),
            event_task: current(),
        });
        tokio::spawn(event_task);
        Ok(EventTaskHandle {
            _priv: (),
        })
    }
}

impl Drop for EventTaskHandle {
    fn drop(&mut self) {
        let mut global_set = GLOBAL_WATCHER_SET.lock().unwrap();
        {
            let global_set = global_set.as_mut().unwrap();
            global_set.event_task.notify();
        }
        *global_set = None;
    }
}

impl Stream for EventWatcher {
    type Item = Event;
    type Error = Arc<failure::Error>;

    fn poll(&mut self) -> Result<Async<Option<Event>>, Arc<failure::Error>> {
        trace!("polling EventWatcher");
        let mut global_set = GLOBAL_WATCHER_SET.lock().unwrap();
        let global_set = global_set.as_mut().unwrap();
        global_set.watcher_tasks[self.key] = current();

        if self.odd_numbered_round != global_set.odd_numbered_round {
            trace!("already polled this round");
            return Ok(Async::NotReady);
        }

        trace!("we haven't polled yet this round (odd = {})", self.odd_numbered_round);
        global_set.num_polled_this_round += 1;
        self.odd_numbered_round ^= true;

        let ret = match global_set.current_event {
            Err(ref e) => Err(e.clone()),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(ref event))) => {
                let mut event = event.clone();
                EVENT_MAP.with(|event_map| {
                    let event_map = event_map.lock().unwrap();
                    let mut event_maps = event_map.iter();
                    loop {
                        event = match event_maps.next() {
                            Some(event_map) => match event_map(event) {
                                Some(event) => event,
                                None => break Ok(Async::NotReady),
                            },
                            None => break Ok(Async::Ready(Some(event))),
                        };
                    }
                })
            },
        };

        trace!("got ret == {:?}", ret);
        trace!("{} of {} watchers have polled", global_set.num_polled_this_round, global_set.watcher_tasks.len());
        if global_set.num_polled_this_round == global_set.watcher_tasks.len() {
            trace!("everyone has polled. waking the event task");
            global_set.current_event = Ok(Async::NotReady);
            global_set.event_task.notify();
        }

        ret
    }
}

pub fn key(key: Key) -> impl Future<Item = (), Error = failure::Error> {
    let event = EventWatcher::new();
    event
    .filter_map(move |event| match event {
        Event::Key(got) if got == key => Some(()),
        _ => None,
    })
    .into_future()
    .map_err(|(e, _)| {
        format_err!("error reading stdin: {}", e)
    })
    .map(|(opt, _)| opt.unwrap())
}

pub fn unsupported<'a>(bytes: &'a [u8]) -> impl Future<Item = (), Error = failure::Error> + 'a {
    let event = EventWatcher::new();
    event
    .filter_map(move |event| match event {
        Event::Unsupported(ref v) if &v[..] == bytes => Some(()),
        _ => None,
    })
    .into_future()
    .map_err(|(e, _)| {
        format_err!("error reading stdin: {}", e)
    })
    .map(|(opt, _)| opt.unwrap())
}

pub fn left_click() -> impl Future<Item = (u16, u16), Error = failure::Error> {
    let event = EventWatcher::new();
    event
    .filter_map(move |event| match event {
        Event::Mouse(MouseEvent::Press(MouseButton::Left, x, y)) => Some((x - 1, y - 1)),
        _ => None,
    })
    .into_future()
    .map_err(|(e, _)| {
        format_err!("error reading stdin: {}", e)
    })
    .map(|(opt, _)| opt.unwrap())
}

