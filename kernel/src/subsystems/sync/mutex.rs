// This file is intentionally left mostly empty.
// All mutex functionality has been moved to the main sync module at:
// /Users/wangbiao/Desktop/project/nos/kernel/src/sync/mod.rs
//
// The subsystems::sync module now re-exports the mutex types from:
// kernel/src/sync/mod.rs via:
// pub use mutex::{Mutex, MutexGuard, MutexIrq, MutexIrqGuard};
//
// This change was made to eliminate duplicate definitions and resolve
// compilation conflicts between the sync and subsystems::sync modules.