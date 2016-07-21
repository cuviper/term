//! # Logger
//!
//! Loggers are thread-safe and reference counted, so can be freely
//! passed around the code.
//!
//! Each logger is built with a set of key-values.
//!
//! Child loggers are build from existing loggers, and copy
//! all the key-values from their parents
//!
//! Loggers form hierarchies sharing a drain. Setting a drain on
//! any logger will change it on all loggers in given hierarchy.
use super::{OwnedKeyValue, Level, BorrowedKeyValue};
use std::sync::Arc;
use std::cell::RefCell;
use crossbeam::sync::ArcCell;

use drain;

use chrono;

thread_local! {
    static TL_BUF: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(128))
}

// TODO: Implement custom clone, that starts with a new buffer
#[derive(Clone)]
/// Logger
pub struct Logger {
    drain: Arc<ArcCell<Box<drain::Drain>>>,
    values: Vec<OwnedKeyValue>,
}

impl Logger {
    /// Build a root logger
    ///
    /// All children and their children and so on form one hierarchy
    /// sharing a common drain.
    ///
    /// Use `o!` macro to help build `values`
    ///
    /// ```
    /// #[macro_use]
    /// extern crate slog;
    ///
    /// fn main() {
    ///     let root = slog::Logger::new_root(o!("key1" => "value1", "key2" => "value2"));
    /// }
    pub fn new_root(values: &[OwnedKeyValue]) -> Logger {
        let drain =
            Arc::new(ArcCell::new(Arc::new(Box::new(drain::discard()) as Box<drain::Drain>)));
        Logger {
            drain: drain,
            values: values.to_vec(),
        }
    }

    /// Build a child logger
    ///
    /// Child logger copies all existing values from the parent.
    ///
    /// All children, their children and so on, form one hierarchy sharing
    /// a common drain.
    ///
    /// Use `o!` macro to help build `values`
    ///
    /// ```
    /// #[macro_use]
    /// extern crate slog;
    ///
    /// fn main() {
    ///     let root = slog::Logger::new_root(o!("key1" => "value1", "key2" => "value2"));
    ///     let log = root.new(o!("key" => "value"));
    /// }

    pub fn new(&self, values: &[OwnedKeyValue]) -> Logger {
        let mut new_values = self.values.clone();
        new_values.extend_from_slice(values);
        Logger {
            drain: self.drain.clone(),
            values: new_values,
        }
    }

    /// Set the drain for logger and it's hierarchy
    pub fn set_drain<D: drain::Drain>(&self, drain: D) {
        let _ = self.drain.set(Arc::new(Box::new(drain)));
    }

    /// Swap the existing drain with a new one
    ///
    /// As the drains are shared between threads, and might still be
    /// referenced `Arc`s are being used to reference-count them.
    pub fn swap_drain(&self, drain: Arc<Box<drain::Drain>>) -> Arc<Box<drain::Drain>> {
        self.drain.set(drain)
    }

    /// Log one logging record
    ///
    /// Use specific logging functions instead.
    pub fn log(&self, lvl: Level, msg: &str, values: &[BorrowedKeyValue]) {

        let mut info = RecordInfo::new(lvl, msg);

        // By default errors in loggers are ignored
        TL_BUF.with(|buf| {
            let mut buf = buf.borrow_mut();
            let _ = self.drain.get().log(&mut *buf, &mut info, self.values.as_slice(), values);
            // TODO: Double check if this will not zero the old bytes as it costs time
            buf.clear();
        });
    }

    /// Log critical level record
    ///
    /// Use `b!` macro to help build `values`
    pub fn critical(&self, msg: &str, values: &[BorrowedKeyValue]) {
        self.log(Level::Critical, msg, values);
    }

    /// Log error level record
    ///
    /// Use `b!` macro to help build `values`
    pub fn error(&self, msg: &str, values: &[BorrowedKeyValue]) {
        self.log(Level::Error, msg, values);
    }

    /// Log warning level record
    ///
    /// Use `b!` macro to help build `values`
    pub fn warn(&self, msg: &str, values: &[BorrowedKeyValue]) {
        self.log(Level::Warning, msg, values);
    }

    /// Log info level record
    ///
    /// Use `b!` macro to help build `values`
    pub fn info(&self, msg: &str, values: &[BorrowedKeyValue]) {
        self.log(Level::Info, msg, values);
    }

    /// Log debug level record
    ///
    /// Use `b!` macro to help build `values`
    pub fn debug(&self, msg: &str, values: &[BorrowedKeyValue]) {
        self.log(Level::Debug, msg, values);
    }

    /// Log trace level record
    ///
    /// Use `b!` macro to help build `values`
    pub fn trace(&self, msg: &str, values: &[BorrowedKeyValue]) {
        self.log(Level::Trace, msg, values);
    }
}

/// Common information about a logging record
pub struct RecordInfo<'a> {
    ts: RefCell<Option<chrono::DateTime<chrono::UTC>>>,
    /// Logging level
    level: Level,
    /// Message
    msg: &'a str,
}

impl<'a> RecordInfo<'a> {
    /// Create a new `RecordInfo` with a current timestamp
    pub fn new(level: Level, msg: &'a str) -> Self {
        RecordInfo {
            ts: RefCell::new(None),
            level: level,
            msg: msg,
        }
    }

    /// Timestamp
    ///
    /// Lazily evaluated timestamp
    pub fn ts(&self) -> chrono::DateTime<chrono::UTC> {
        let mut ts = self.ts.borrow_mut();
        match *ts {
            None => {
                let now = chrono::UTC::now();
                *ts = Some(now);
                now
            },
            Some(ts) => ts
        }
    }


    /// Set timestamp
    pub fn set_ts(&self, ts : chrono::DateTime<chrono::UTC>) {
        *self.ts.borrow_mut() = Some(ts);
    }

    /// Get a log record message
    pub fn msg(&self) -> &str {
        self.msg
    }


    /// Get record logging level
    pub fn level(&self) -> Level {
        self.level
    }
}
