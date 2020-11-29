use super::*;
use std::net::TcpStream;
use log::{Log, Record, Metadata, LevelFilter, SetLoggerError};

lazy_static! {
    static ref TCP_LOGGER: TcpLogger = {
        let stream_opt = Mutex::new(TcpStream::connect("127.0.0.1:45666").ok());
        TcpLogger { stream_opt }
    };
}

struct TcpLogger {
    stream_opt: Mutex<Option<TcpStream>>,
}

impl Log for TcpLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        let stream_opt = self.stream_opt.lock().unwrap();
        stream_opt.is_some()
    }

    fn log(&self, record: &Record) {
        let mut stream_opt = self.stream_opt.lock().unwrap();
        if let Some(stream) = stream_opt.as_mut() {
            let _ = writeln!(stream, "{}", record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&*TCP_LOGGER)?;
    log::set_max_level(LevelFilter::Trace);
    Ok(())
}
