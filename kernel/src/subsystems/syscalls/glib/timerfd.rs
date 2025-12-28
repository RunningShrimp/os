#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! GLib timerfd re-exports
//!
//! This module bridges the core `timerfd` implementation into the
//! `glib` namespace so that `glib::timerfd` can be used without
//! duplicating logic.

pub use crate::subsystems::syscalls::timerfd::*;



