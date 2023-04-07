#[derive(Debug, Clone, PartialEq)]
pub enum LogOutput<'a> {
    Log(log::Level),
    LogTarget(log::Level, String),
    StdOut,
    StdErr,
    Path(&'a str),
}

impl From<log::Level> for LogOutput<'static> {
    fn from(value: log::Level) -> Self {
        Self::Log(value)
    }
}

pub trait Loggable {
    fn log(&self, output: &LogOutput);
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($msg:tt)*) => {{
        match $level {
            LogOutput::StdOut => println!($($msg)*),
            LogOutput::StdErr => eprintln!($($msg)*),
            LogOutput::Log(level) => log::log!(level.clone(), $($msg)*),
            LogOutput::LogTarget(level, target) => {log::log!(target: target, level.clone(), $($msg)*)}
            LogOutput::Path(path) => {
                let message = format!($($msg)*);

                fn write_to_path(message: &str, path: &str) {
                    let file = std::fs::File::options().append(true).create(true).open(path);

                    match file {
                        Ok(mut file) => {
                            use std::io::Write;
                            file.write_all(message.as_bytes()).ok();
                            file.write_all(b"\n").ok();
                        }
                        Err(e) => {
                            eprintln!("Could not open log file: {:?}", e);
                        }
                    }
                }

                write_to_path(&message, path);
            }
        }
    }};
}
