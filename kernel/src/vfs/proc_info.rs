//! Process information (/proc/[pid])

extern crate alloc;
use alloc::{boxed::Box, string::String, string::ToString, sync::Arc, vec::Vec};
use crate::subsystems::sync::Mutex;
use crate::vfs::{
    error::*,
    types::*,
    fs::InodeOps,
    dir::DirEntry,
};

/// Process information inode
pub struct ProcInfoInode {
    pid: usize,
    attr: Mutex<FileAttr>,
}

impl ProcInfoInode {
    /// Create an inode for a specific process
    pub fn create_for_pid(pid: usize) -> VfsResult<Arc<dyn InodeOps>> {
        // Verify process exists
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        if proc_table.find_ref(pid as i32).is_none() {
            return Err(VfsError::NotFound);
        }
        drop(proc_table);
        
        Ok(Arc::new(Self {
            pid,
            attr: Mutex::new(FileAttr {
                ino: pid as u64 + 10000,
                mode: FileMode(FileMode::S_IFDIR | 0o555),
                nlink: 2,
                ..Default::default()
            }),
        }))
    }
    
    /// Generate process status information
    fn generate_status(&self) -> String {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        if let Some(proc) = proc_table.find_ref(self.pid as i32) {
            // Proc结构体没有name字段，使用PID作为标识
            let name = format!("process_{}", self.pid);
            let state = match proc.state {
                crate::process::manager::ProcState::Running => "R",
                crate::process::manager::ProcState::Sleeping => "S",
                crate::process::manager::ProcState::Runnable => "R",
                crate::process::manager::ProcState::Zombie => "Z",
                _ => "?",
            };
            
            format!(
                "Name:\t{}\n\
                 State:\t{}\n\
                 Pid:\t{}\n\
                 PPid:\t{}\n\
                 Uid:\t0\n\
                 Gid:\t0\n",
                name, state, self.pid, 0
            )
        } else {
            String::new()
        }
    }
    
    /// Generate process command line
    fn generate_cmdline(&self) -> String {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        if proc_table.find_ref(self.pid as i32).is_some() {
            // Proc结构体没有name字段，使用PID作为标识
            format!("process_{}\0", self.pid)
        } else {
            String::new()
        }
    }
    
    /// Generate process memory information
    fn generate_statm(&self) -> String {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        if let Some(proc) = proc_table.find_ref(self.pid as i32) {
            let size = proc.sz;
            format!("{} 0 0 0 0 0 0\n", size / 4096) // Size in pages
        } else {
            String::new()
        }
    }
}

/// Format /proc/[pid]/stat content
/// Linux /proc/[pid]/stat format has 52 fields separated by spaces
fn format_proc_stat(pid: usize) -> String {
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find_ref(pid as crate::process::Pid) {
        // Proc结构体没有name字段，使用PID作为标识
        let name = format!("process_{}", pid);
        let state = match proc.state {
            crate::process::manager::ProcState::Running => "R",
            crate::process::manager::ProcState::Sleeping => "S",
            crate::process::manager::ProcState::Runnable => "R",
            crate::process::manager::ProcState::Zombie => "Z",
            _ => "?",
        };
        
        // Build stat fields using Vec and join to avoid long format! macro
        let mut fields = Vec::new();
        fields.push(pid.to_string());           // 1: pid
        fields.push(format!("({})", name));    // 2: comm (command name in parentheses)
        fields.push(state.to_string());        // 3: state
        fields.push("0".to_string());          // 4: ppid
        fields.push("0".to_string());          // 5: pgrp
        fields.push("0".to_string());          // 6: session
        fields.push("0".to_string());          // 7: tty_nr
        fields.push("0".to_string());          // 8: tpgid
        fields.push("0".to_string());          // 9: flags
        fields.push("0".to_string());          // 10: minflt
        fields.push("0".to_string());          // 11: cminflt
        fields.push("0".to_string());          // 12: majflt
        fields.push("0".to_string());          // 13: cmajflt
        fields.push("0".to_string());          // 14: utime
        fields.push("0".to_string());          // 15: stime
        fields.push("0".to_string());          // 16: cutime
        fields.push("0".to_string());          // 17: cstime
        fields.push("0".to_string());          // 18: priority
        fields.push("0".to_string());          // 19: nice
        fields.push("0".to_string());          // 20: num_threads
        fields.push("0".to_string());          // 21: itrealvalue
        fields.push("0".to_string());          // 22: starttime
        fields.push((proc.sz / 4096).to_string()); // 23: vsize (in pages)
        fields.push("0".to_string());          // 24: rss
        fields.push("0".to_string());          // 25: rsslim
        fields.push("0".to_string());          // 26: startcode
        fields.push("0".to_string());          // 27: endcode
        fields.push("0".to_string());          // 28: startstack
        fields.push("0".to_string());          // 29: kstkesp
        fields.push("0".to_string());          // 30: kstkeip
        fields.push("0".to_string());          // 31: signal
        fields.push("0".to_string());          // 32: blocked
        fields.push("0".to_string());          // 33: sigignore
        fields.push("0".to_string());          // 34: sigcatch
        fields.push("0".to_string());          // 35: wchan
        fields.push("0".to_string());          // 36: nswap
        fields.push("0".to_string());          // 37: cnswap
        fields.push("0".to_string());          // 38: exit_signal
        fields.push("0".to_string());          // 39: processor
        fields.push("0".to_string());          // 40: rt_priority
        fields.push("0".to_string());          // 41: policy
        fields.push("0".to_string());          // 42: delayacct_blkio_ticks
        fields.push("0".to_string());          // 43: guest_time
        fields.push("0".to_string());          // 44: cguest_time
        fields.push("0".to_string());          // 45: start_data
        fields.push("0".to_string());          // 46: end_data
        fields.push("0".to_string());          // 47: start_brk
        fields.push("0".to_string());          // 48: arg_start
        fields.push("0".to_string());          // 49: arg_end
        fields.push("0".to_string());          // 50: env_start
        fields.push("0".to_string());          // 51: env_end
        fields.push("0".to_string());          // 52: exit_code
        
        fields.join(" ") + "\n"
    } else {
        String::new()
    }
}

impl InodeOps for ProcInfoInode {
    fn getattr(&self) -> VfsResult<FileAttr> {
        Ok(self.attr.lock().clone())
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn InodeOps>> {
        match name {
            "status" => {
                let pid = self.pid;
                Ok(Arc::new(super::fs::ProcFsInode::new_file(
                    pid as u64 + 20000,
                    Box::new(move || {
                        let proc_table = crate::process::manager::PROC_TABLE.lock();
                        if let Some(proc) = proc_table.find_ref(pid as crate::process::Pid) {
                            // Proc结构体没有name字段，使用PID作为标识
                            let name = format!("process_{}", pid);
                            let state = match proc.state {
                                crate::process::manager::ProcState::Running => "R",
                                crate::process::manager::ProcState::Sleeping => "S",
                                crate::process::manager::ProcState::Runnable => "R",
                                crate::process::manager::ProcState::Zombie => "Z",
                                _ => "?",
                            };
                            format!(
                                "Name:\t{}\n\
                                 State:\t{}\n\
                                 Pid:\t{}\n\
                                 PPid:\t{}\n\
                                 Uid:\t0\n\
                                 Gid:\t0\n",
                                name, state, pid, 0
                            )
                        } else {
                            String::new()
                        }
                    }),
                )))
            }
            "cmdline" => {
                let pid = self.pid;
                Ok(Arc::new(super::fs::ProcFsInode::new_file(
                    pid as u64 + 20001,
                    Box::new(move || {
                        let proc_table = crate::process::manager::PROC_TABLE.lock();
                        if proc_table.find_ref(pid as crate::process::Pid).is_some() {
                            // Proc结构体没有name字段，使用PID作为标识
                            format!("process_{}\0", pid)
                        } else {
                            String::new()
                        }
                    }),
                )))
            }
            "statm" => {
                let pid = self.pid;
                Ok(Arc::new(super::fs::ProcFsInode::new_file(
                    pid as u64 + 20002,
                    Box::new(move || {
                        let proc_table = crate::process::manager::PROC_TABLE.lock();
                        if let Some(proc) = proc_table.find_ref(pid as crate::process::Pid) {
                            let size = proc.sz;
                            format!("{} 0 0 0 0 0 0\n", size / 4096)
                        } else {
                            String::new()
                        }
                    }),
                )))
            }
            "stat" => {
                let pid = self.pid;
                Ok(Arc::new(super::fs::ProcFsInode::new_file(
                    pid as u64 + 20003,
                    Box::new(move || {
                        format_proc_stat(pid)
                    }),
                )))
            }
            _ => Err(VfsError::NotFound),
        }
    }
}
