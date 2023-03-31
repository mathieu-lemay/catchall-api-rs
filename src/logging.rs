use std::io::Write;

use env_logger::fmt::{Color, Formatter, Style};
use env_logger::{Builder, Env};
use log::{Level, Record};

struct LogFormatter<'a> {
    buf: &'a mut Formatter,
    record: &'a Record<'a>,
}

impl<'a> LogFormatter<'a> {
    fn new(buf: &'a mut Formatter, record: &'a Record) -> Self {
        Self { buf, record }
    }

    fn write(&mut self) -> Result<(), std::io::Error> {
        let msg_style = self.get_message_style();
        let ts_style = self.get_timestamp_style();
        let mod_style = self.get_module_style();

        let timestamp = self.buf.timestamp_millis();
        let mod_path = self.record.module_path().unwrap_or("main");

        writeln!(
            self.buf,
            "{} {} {}",
            ts_style.value(timestamp),
            mod_style.value(format!("{}:", mod_path)),
            msg_style.value(self.record.args()),
        )
    }

    #[inline]
    fn get_timestamp_style(&self) -> Style {
        let mut style = self.buf.style();
        style.set_color(Color::Green);

        style
    }

    #[inline]
    fn get_module_style(&self) -> Style {
        let mut style = self.buf.style();
        style.set_color(Color::Blue);

        style
    }

    #[inline]
    fn get_message_style(&self) -> Style {
        let color = match self.record.level() {
            Level::Trace => Some(Color::Cyan),
            Level::Debug => Some(Color::Green),
            Level::Info => None,
            Level::Warn => Some(Color::Yellow),
            Level::Error => Some(Color::Red),
        };

        let mut style = self.buf.style();
        if let Some(c) = color {
            style.set_color(c);
        }

        if self.record.level() == Level::Error {
            style.set_bold(true);
        }

        style
    }
}

pub(crate) fn init_logger() {
    let env = Env::default().filter_or("CATCHALL_API_LOG_LEVEL", "info");

    Builder::from_env(env)
        .format_timestamp_nanos()
        .format(|buf, record| LogFormatter::new(buf, record).write())
        .init();
}
