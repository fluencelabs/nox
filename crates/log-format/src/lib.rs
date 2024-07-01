/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use nu_ansi_term::{Color, Style};
use std::fmt;
use tracing::{Event, Level, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::registry::LookupSpan;

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Format<T = SystemTime> {
    pub(crate) timer: T,
    pub(crate) display_timestamp: bool,
    pub(crate) display_target: bool,
    pub(crate) display_level: bool,
    pub(crate) display_span_list: bool,
}

impl Default for Format<SystemTime> {
    fn default() -> Self {
        Format {
            timer: SystemTime,
            display_timestamp: true,
            display_target: true,
            display_level: true,
            display_span_list: false,
        }
    }
}

#[allow(unused)]
impl<T> Format<T> {
    /// Use the given [`timer`] for log message timestamps.
    ///
    /// See [`time` module] for the provided timer implementations.
    ///
    /// Note that using the `"time"` feature flag enables the
    /// additional time formatters [`UtcTime`] and [`LocalTime`], which use the
    /// [`time` crate] to provide more sophisticated timestamp formatting
    /// options.
    ///
    /// [`timer`]: super::time::FormatTime
    /// [`time` module]: mod@super::time
    /// [`UtcTime`]: super::time::UtcTime
    /// [`LocalTime`]: super::time::LocalTime
    /// [`time` crate]: https://docs.rs/time/0.3
    pub fn with_timer<T2>(self, timer: T2) -> Format<T2> {
        Format {
            timer,
            display_target: self.display_target,
            display_timestamp: self.display_timestamp,
            display_level: self.display_level,
            display_span_list: self.display_span_list,
        }
    }

    /// Do not emit timestamps with log messages.
    pub fn without_time(self) -> Format<()> {
        Format {
            timer: (),
            display_timestamp: false,
            display_target: self.display_target,
            display_level: self.display_level,
            display_span_list: self.display_span_list,
        }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> Format<T> {
        Format {
            display_target,
            ..self
        }
    }

    /// Sets whether or not an event's spans is displayed.
    pub fn with_display_span_list(self, display_span_list: bool) -> Format<T> {
        Format {
            display_span_list,
            ..self
        }
    }

    /// Sets whether or not an event's level is displayed.
    pub fn with_level(self, display_level: bool) -> Format<T> {
        Format {
            display_level,
            ..self
        }
    }

    #[inline]
    fn format_timestamp(&self, writer: &mut Writer<'_>) -> fmt::Result
    where
        T: FormatTime,
    {
        // If timestamps are disabled, do nothing.
        if !self.display_timestamp {
            return Ok(());
        }

        // If ANSI color codes are enabled, format the timestamp with ANSI
        // colors.
        {
            if writer.has_ansi_escapes() {
                let style = nu_ansi_term::Style::new().dimmed();
                write!(writer, "{}", style.prefix())?;

                // If getting the timestamp failed, don't bail --- only bail on
                // formatting errors.
                if self.timer.format_time(writer).is_err() {
                    writer.write_str("<unknown time>")?;
                }

                write!(writer, "{} ", style.suffix())?;
                return Ok(());
            }
        }

        // Otherwise, just format the timestamp without ANSI formatting.
        // If getting the timestamp failed, don't bail --- only bail on
        // formatting errors.
        if self.timer.format_time(writer).is_err() {
            writer.write_str("<unknown time>")?;
        }
        writer.write_char(' ')
    }
}

impl<S, N, T> FormatEvent<S, N> for Format<T>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let normalized_meta = event.normalized_metadata();
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());

        self.format_timestamp(&mut writer)?;

        if self.display_level {
            let fmt_level = { FmtLevel::new(meta.level(), writer.has_ansi_escapes()) };
            write!(writer, "{} ", fmt_level)?;
        }

        let dimmed = if writer.has_ansi_escapes() {
            Style::new().dimmed()
        } else {
            Style::new()
        };

        if self.display_span_list {
            if let Some(scope) = ctx.event_scope() {
                let bold = if writer.has_ansi_escapes() {
                    Style::new().bold()
                } else {
                    Style::new()
                };

                let mut spans = scope.from_root().peekable();

                if spans.peek().is_some() {
                    write!(writer, "{}", bold.paint("{"))?;
                }
                let mut first_write = true;

                for span in spans {
                    let ext = span.extensions();
                    if let Some(fields) = &ext.get::<FormattedFields<N>>() {
                        if !fields.is_empty() {
                            if first_write {
                                write!(writer, " ")?;
                                first_write = false;
                            }
                            write!(writer, "{}", fields)?;
                            write!(writer, " ")?;
                        }
                    }
                }
                write!(writer, "{}", bold.paint("}"))?;
            };
            write!(writer, " ")?;
        };

        if self.display_target {
            write!(
                writer,
                "{}{} ",
                dimmed.paint(meta.target()),
                dimmed.paint(":")
            )?;
        }

        ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

const TRACE_STR: &str = "TRACE";
const DEBUG_STR: &str = "DEBUG";
const INFO_STR: &str = " INFO";
const WARN_STR: &str = " WARN";
const ERROR_STR: &str = "ERROR";

struct FmtLevel<'a> {
    level: &'a Level,
    ansi: bool,
}

impl<'a> FmtLevel<'a> {
    pub(crate) fn new(level: &'a Level, ansi: bool) -> Self {
        Self { level, ansi }
    }
}

impl<'a> fmt::Display for FmtLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ansi {
            match *self.level {
                Level::TRACE => write!(f, "{}", Color::Purple.paint(TRACE_STR)),
                Level::DEBUG => write!(f, "{}", Color::Blue.paint(DEBUG_STR)),
                Level::INFO => write!(f, "{}", Color::Green.paint(INFO_STR)),
                Level::WARN => write!(f, "{}", Color::Yellow.paint(WARN_STR)),
                Level::ERROR => write!(f, "{}", Color::Red.paint(ERROR_STR)),
            }
        } else {
            match *self.level {
                Level::TRACE => f.pad(TRACE_STR),
                Level::DEBUG => f.pad(DEBUG_STR),
                Level::INFO => f.pad(INFO_STR),
                Level::WARN => f.pad(WARN_STR),
                Level::ERROR => f.pad(ERROR_STR),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::{fmt, io};

    use subscriber::with_default;
    use tracing::{info_span, subscriber};
    use tracing_subscriber::fmt::format::Writer;
    use tracing_subscriber::fmt::time::FormatTime;
    use tracing_subscriber::fmt::{MakeWriter, Subscriber};

    use crate::Format;

    #[derive(Clone, Debug)]
    struct MockWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    #[derive(Clone, Debug)]
    struct MockMakeWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl MockMakeWriter {
        fn new() -> Self {
            Self {
                buf: Arc::new(Mutex::new(Vec::new())),
            }
        }
        fn get_content(&self) -> String {
            let buf = self.buf.lock().unwrap();
            std::str::from_utf8(&buf[..]).unwrap().to_owned()
        }
    }

    impl MockWriter {
        fn new(buf: Arc<Mutex<Vec<u8>>>) -> Self {
            Self { buf }
        }
    }

    impl io::Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.buf.lock().unwrap().flush()
        }
    }

    impl<'a> MakeWriter<'a> for MockMakeWriter {
        type Writer = MockWriter;

        fn make_writer(&'a self) -> Self::Writer {
            MockWriter::new(self.buf.clone())
        }
    }

    pub(crate) struct MockTime;
    impl FormatTime for MockTime {
        fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
            write!(w, "fake time")
        }
    }

    #[test]
    fn test_simple_info() {
        let make_writer = MockMakeWriter::new();

        let format = Format::default().with_timer(MockTime);
        let subscriber = Subscriber::builder()
            .with_ansi(false)
            .event_format(format)
            .with_writer(make_writer.clone())
            .finish();

        with_default(subscriber, || {
            tracing::info!("info message");
        });

        let expected = "fake time  INFO log_format::tests: info message\n";

        assert_eq!(expected, make_writer.get_content());
    }

    #[test]
    fn test_asni_info() {
        let writer = MockMakeWriter::new();

        let format = Format::default().with_timer(MockTime);
        let subscriber = Subscriber::builder()
            .with_ansi(true)
            .event_format(format)
            .with_writer(writer.clone())
            .finish();

        with_default(subscriber, || {
            tracing::info!("info message");
        });

        let expected =  "\u{1b}[2mfake time\u{1b}[0m \u{1b}[32m INFO\u{1b}[0m \u{1b}[2mlog_format::tests\u{1b}[0m\u{1b}[2m:\u{1b}[0m info message\n";

        assert_eq!(expected, writer.get_content());
    }

    #[test]
    fn test_simple_info_with_span() {
        let writer = MockMakeWriter::new();

        let format = Format::default()
            .with_timer(MockTime)
            .with_display_span_list(true);
        let subscriber = Subscriber::builder()
            .with_ansi(false)
            .event_format(format)
            .with_writer(writer.clone())
            .finish();

        with_default(subscriber, || {
            let span_1 = info_span!("test", id = "test-id", id_x = "test-id-x");
            let span_2 = info_span!(parent: &span_1, "test", id = "test-id-x", id_2 ="test-id-2", id_2="test-id-3");
            tracing::info!(parent: &span_2, "info message");
        });

        let expected = "fake time  INFO { id=\"test-id\" id_x=\"test-id-x\" id=\"test-id-x\" id_2=\"test-id-2\" id_2=\"test-id-3\" } log_format::tests: info message\n";

        assert_eq!(expected, writer.get_content());
    }

    #[test]
    fn test_simple_info_without_spans() {
        let writer = MockMakeWriter::new();

        let format = Format::default()
            .with_timer(MockTime)
            .with_display_span_list(true);
        let subscriber = Subscriber::builder()
            .with_ansi(false)
            .event_format(format)
            .with_writer(writer.clone())
            .finish();

        with_default(subscriber, || {
            tracing::info!("info message");
        });

        let expected = "fake time  INFO  log_format::tests: info message\n";

        assert_eq!(expected, writer.get_content());
    }
}
