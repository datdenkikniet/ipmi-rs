#[derive(Debug, Clone, PartialEq)]
pub enum LogOutput {
    Log(log::Level),
    LogTarget(log::Level, String),
    StdOut,
    StdErr,
}

impl From<log::Level> for LogOutput {
    fn from(value: log::Level) -> Self {
        Self::Log(value)
    }
}

impl LogOutput {
    fn print(&self, msg: &str) {
        match self {
            LogOutput::Log(level) => log::log!(level.clone(), "{}", msg),
            LogOutput::LogTarget(level, target) => {
                log::log!(target: target, level.clone(), "{}", msg)
            }
            LogOutput::StdOut => println!("{}", msg),
            LogOutput::StdErr => eprintln!("{}", msg),
        }
    }
}

#[derive(Debug)]
pub struct LogItem {
    level: usize,
    title: String,
    value: Option<String>,
}

impl LogItem {
    pub fn new<T: Into<String>, V: Into<String>>(level: usize, title: T, value: Option<V>) -> Self {
        Self {
            level,
            title: title.into(),
            value: value.map(Into::into),
        }
    }
}

impl<T: ToString, V: ToString> From<(usize, T, V)> for LogItem {
    fn from((level, title, value): (usize, T, V)) -> Self {
        Self::new(level, title.to_string(), Some(value.to_string()))
    }
}

impl<T: ToString> From<(usize, T)> for LogItem {
    fn from((level, value): (usize, T)) -> Self {
        Self::new::<_, String>(level, value.to_string(), None)
    }
}

pub struct Logger;

impl Logger {
    pub fn log<T>(output: &LogOutput, loggable: &T)
    where
        T: Loggable,
    {
        Self::log_impl(output, &loggable.into_log())
    }

    fn log_impl(output: &LogOutput, items: &[LogItem]) {
        // TODO: support log items with more than 2 steps of log levels
        items.iter().next().map(|v| output.print(&v.title));

        let right_align = items
            .iter()
            .skip(1)
            .map(|v| v.title.len())
            .max()
            .unwrap_or(0);

        items.iter().skip(1).for_each(|i| {
            let LogItem {
                level,
                title,
                value,
            } = i;

            let front_padding: String = (0..level * 2).map(|_| ' ').collect();

            let (value, value_padding) = if let Some(value) = value {
                let value_padding: String = (0..(right_align - title.len())).map(|_| ' ').collect();
                (value.as_str(), value_padding)
            } else {
                ("", String::new())
            };

            let message = format!("{front_padding}{title}: {value_padding}{value}");
            output.print(&message);
        })
    }
}

pub trait Loggable {
    fn into_log(&self) -> Vec<LogItem>;
}

#[macro_export]
macro_rules ! log_vec {
    [$($msg:tt)*] => {
        crate::to_log!(vec: $($msg)*)
    }
}

#[macro_export]
macro_rules! to_log {
    ([$($array:tt)*],) => {
        vec![$($array)*]
    };

    ([$($array:tt)*], ($level:literal, $title:expr, $value:expr)) => {
        crate::to_log!([$($array)* ($level, $title, $value).into(),],)
    };

    ([$($array:tt)*], ($level:literal, $title:expr)) => {
        crate::to_log!([$($array)* ($level, $title, "").into(),],)
    };

    ([$($array:tt)*], ($level:literal, $title:expr, $value:expr), $($msg:tt)*) => {
        crate::to_log!([$($array)* ($level, $title, $value).into(),], $($msg)*)
    };

    ([$($array:tt)*], ($level:literal, $title:expr), $($msg:tt)*) => {
        crate::to_log!([$($array)* ($level, $title, "").into(),], $($msg)*)
    };

    (vec: $($msg:tt)*) => {
        crate::to_log!([], $($msg)*)
    };
}
