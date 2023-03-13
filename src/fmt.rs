#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogOutput {
    Log(log::Level),
    StdOut,
    StdErr,
}

impl From<log::Level> for LogOutput {
    fn from(value: log::Level) -> Self {
        Self::Log(value)
    }
}

pub trait Loggable {
    fn log(&self, output: LogOutput);
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($msg:tt)*) => {{
        let level: LogOutput = $level.into();
        match level {
            LogOutput::StdOut => println!($($msg)*),
            LogOutput::StdErr => eprintln!($($msg)*),
            LogOutput::Log(level) => log::log!(level, $($msg)*),
        }
    }};
}
