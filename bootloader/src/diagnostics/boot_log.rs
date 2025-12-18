// Boot logging system for diagnostics and debugging

use alloc::vec::Vec;
use alloc::string::String;

pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

pub struct BootLog {
    entries: Vec<String>,
    max_entries: usize,
}

impl BootLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 256,
        }
    }

    pub fn log(&mut self, message: &str) {
        if self.entries.len() < self.max_entries {
            self.entries.push(String::from(message));
        }
    }

    pub fn log_info(&mut self, info: &str) {
        self.log(info);
    }

    pub fn log_error(&mut self, error: &str) {
        let mut msg = String::from("ERROR: ");
        msg.push_str(error);
        self.log(&msg);
    }

    pub fn log_warn(&mut self, warn: &str) {
        let mut msg = String::from("WARN: ");
        msg.push_str(warn);
        self.log(&msg);
    }

    pub fn print_all(&self) {
        crate::drivers::console::write_str("=== Boot Log ===\n");
        for entry in &self.entries {
            crate::drivers::console::write_str(entry.as_str());
            crate::drivers::console::write_str("\n");
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for BootLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Global boot logger
pub static mut BOOT_LOGGER: Option<BootLog> = None;

pub fn init_logger() {
    unsafe {
        BOOT_LOGGER = Some(BootLog::new());
    }
}

pub fn log_message(msg: &str) {
    unsafe {
        match &mut *(&raw mut BOOT_LOGGER) {
            Some(logger) => logger.log(msg),
            None => {}
        }
    }
}

pub fn log_error(error: &str) {
    unsafe {
        match &mut *(&raw mut BOOT_LOGGER) {
            Some(logger) => logger.log_error(error),
            None => {}
        }
    }
}

pub fn log_warn(warn: &str) {
    unsafe {
        match &mut *(&raw mut BOOT_LOGGER) {
            Some(logger) => logger.log_warn(warn),
            None => {}
        }
    }
}

pub fn dump_log() {
    unsafe {
        match &*(&raw const BOOT_LOGGER) {
            Some(logger) => logger.print_all(),
            None => {}
        }
    }
}
