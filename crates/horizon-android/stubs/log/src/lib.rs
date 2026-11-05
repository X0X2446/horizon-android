//! Stub for host-build verification. The real crate comes from crates.io.
#![allow(dead_code)]

pub trait Log: Send + Sync {
    fn enabled(&self, metadata: &Metadata) -> bool;
    fn log(&self, record: &Record);
    fn flush(&self);
}

pub struct Metadata;
pub struct Record;
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

pub fn max_level() -> Level {
    Level::Info
}
