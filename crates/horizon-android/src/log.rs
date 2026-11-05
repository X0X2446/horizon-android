//! The Android log bridge: `log` crate records to `logcat`.
//!
//! Only built for Android. The game's own logging writes to its log files; this
//! bridge is for the launcher's diagnostics (JNI calls, the port's own messages).

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

#[repr(C)]
enum AndroidLogPriority {
    Unknown = 0,
    Debug = 3,
    Info = 4,
    Warn = 5,
    Error = 6,
    Fatal = 7,
    Silent = 8,
}

extern "C" {
    fn __android_log_print(prio: c_int, tag: *const c_char, fmt: *const c_char, ...) -> c_int;
}

/// Log through `logcat` under the given tag.
pub fn print(prio: AndroidLogPriority, tag: &str, message: &str) {
    let tag = match CString::new(tag) {
        Ok(t) => t,
        Err(_) => return,
    };
    // `__android_log_print`'s format string must be valid UTF-8 C; `%s` for the
    // message keeps multi-line output intact.
    let fmt = CString::new("%s").unwrap();
    let message = match CString::new(message) {
        Ok(m) => m,
        Err(_) => return,
    };
    unsafe {
        __android_log_print(
            prio as c_int,
            tag.as_ptr(),
            fmt.as_ptr(),
            message.as_ptr(),
        );
    }
}

pub fn debug(tag: &str, message: &str) {
    print(AndroidLogPriority::Debug, tag, message);
}

pub fn info(tag: &str, message: &str) {
    print(AndroidLogPriority::Info, tag, message);
}

pub fn warn(tag: &str, message: &str) {
    print(AndroidLogPriority::Warn, tag, message);
}

pub fn error(tag: &str, message: &str) {
    print(AndroidLogPriority::Error, tag, message);
}

/// The `log` crate's logger: every record reaches `logcat`.
pub struct LogcatLogger {
    tag: String,
}

impl LogcatLogger {
    pub fn new(tag: impl Into<String>) -> Self {
        Self { tag: tag.into() }
    }
}

impl log::Log for LogcatLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        let level = match record.level() {
            log::Level::Error => AndroidLogPriority::Error,
            log::Level::Warn => AndroidLogPriority::Warn,
            log::Level::Info => AndroidLogPriority::Info,
            log::Level::Debug => AndroidLogPriority::Debug,
            log::Level::Trace => AndroidLogPriority::Debug,
        };
        print(level, &self.tag, record.args().to_string());
    }

    fn flush(&self) {}
}
