//! Pipe implementation for inter-process communication
//!
//! Pipes provide a unidirectional data channel that can be used
//! for communication between processes.

extern crate alloc;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;

use crate::mm::PAGE_SIZE;
use crate::sync::Sleeplock;
use crate::file::{FILE_TABLE, File, FileType};

/// Pipe buffer size
pub const PIPE_SIZE: usize = PAGE_SIZE;

/// Pipe structure
pub struct Pipe {
    pub data: [u8; PIPE_SIZE],
    pub nread: usize,
    pub nwrite: usize,
    pub readopen: bool,
    pub writeopen: bool,
    read_subs: BTreeMap<usize, u32>,
    write_subs: BTreeMap<usize, u32>,
}

impl Pipe {
    /// Create a new pipe
    pub fn new() -> Self {
        Self {
            data: [0; PIPE_SIZE],
            nread: 0,
            nwrite: 0,
            readopen: true,
            writeopen: true,
            read_subs: BTreeMap::new(),
            write_subs: BTreeMap::new(),
        }
    }
    
    /// Close the read end of the pipe
    pub fn close_read(&mut self) {
        self.readopen = false;
        for (&chan, _) in self.write_subs.iter() { crate::process::wakeup(chan); }
    }
    
    /// Close the write end of the pipe
    pub fn close_write(&mut self) {
        self.writeopen = false;
        for (&chan, _) in self.read_subs.iter() { crate::process::wakeup(chan); }
    }
    
    /// Write to the pipe
    pub fn write(&mut self, data: &[u8]) -> Result<usize, ()> {
        let mut written = 0;
        
        for &byte in data {
            // Wait for space in the buffer
            while self.nwrite == self.nread + PIPE_SIZE {
                if !self.readopen {
                    return Err(());
                }
                break;
            }
            
            if self.nwrite >= self.nread + PIPE_SIZE {
                break;
            }
            
            self.data[self.nwrite % PIPE_SIZE] = byte;
            self.nwrite += 1;
            written += 1;
        }
        
        // Wakeup readers handled at caller
        Ok(written)
    }
    
    /// Read from the pipe
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        if self.nread == self.nwrite && self.writeopen {
            // Caller will handle blocking
        }
        
        let mut nread = 0;
        for byte in buf.iter_mut() {
            if self.nread == self.nwrite {
                break;
            }
            *byte = self.data[self.nread % PIPE_SIZE];
            self.nread += 1;
            nread += 1;
        }
        
        // Wakeup writers handled at caller
        Ok(nread)
    }

    pub fn subscribe_read(&mut self, chan: usize) { let c = self.read_subs.get(&chan).cloned().unwrap_or(0); self.read_subs.insert(chan, c+1); }

    pub fn subscribe_write(&mut self, chan: usize) { let c = self.write_subs.get(&chan).cloned().unwrap_or(0); self.write_subs.insert(chan, c+1); }

    pub fn unsubscribe_read(&mut self, chan: usize) { if let Some(c) = self.read_subs.get_mut(&chan) { if *c>1 { *c-=1; } else { self.read_subs.remove(&chan); } } }

    pub fn unsubscribe_write(&mut self, chan: usize) { if let Some(c) = self.write_subs.get_mut(&chan) { if *c>1 { *c-=1; } else { self.write_subs.remove(&chan); } } }

    pub fn notify_read_ready(&self) { for (&chan, _) in self.read_subs.iter() { crate::process::wakeup(chan); } }

    pub fn notify_write_ready(&self) { for (&chan, _) in self.write_subs.iter() { crate::process::wakeup(chan); } }
    
    /// Check if pipe is readable
    pub fn is_readable(&self) -> bool {
        self.nread < self.nwrite || self.writeopen
    }
    
    /// Check if pipe is writable  
    pub fn is_writable(&self) -> bool {
        self.nwrite < self.nread + PIPE_SIZE || self.readopen
    }
    
    /// Get number of bytes available to read
    pub fn available(&self) -> usize {
        self.nwrite - self.nread
    }
}

impl Default for Pipe {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipe allocation result
pub fn pipe_alloc() -> Option<(usize, usize)> {
    let pipe = Arc::new(Sleeplock::new(Pipe::new()));
    let mut table = FILE_TABLE.lock();
    let r = table.alloc()?;
    let w = match table.alloc() {
        Some(idx) => idx,
        None => {
            table.close(r);
            return None;
        }
    };
    if let Some(f) = table.get_mut(r) {
        f.ftype = FileType::Pipe;
        f.ref_count = 1;
        f.readable = true;
        f.writable = false;
        f.pipe = Some(pipe.clone());
    }
    if let Some(f) = table.get_mut(w) {
        f.ftype = FileType::Pipe;
        f.ref_count = 1;
        f.readable = false;
        f.writable = true;
        f.pipe = Some(pipe);
    }
    Some((r, w))
}

pub fn pipe_write(fd: usize, buf: &[u8]) -> isize {
    let mut table = FILE_TABLE.lock();
    match table.get_mut(fd) {
        Some(f) if f.ftype == FileType::Pipe && f.writable => {
            if let Some(ref p) = f.pipe {
                let mut guard = p.lock();
                match guard.write(buf) {
                    Ok(n) => n as isize,
                    Err(_) => -1,
                }
            } else {
                -1
            }
        }
        _ => -1,
    }
}

pub fn pipe_read(fd: usize, buf: &mut [u8]) -> isize {
    let mut table = FILE_TABLE.lock();
    match table.get_mut(fd) {
        Some(f) if f.ftype == FileType::Pipe && f.readable => {
            if let Some(ref p) = f.pipe {
                let mut guard = p.lock();
                match guard.read(buf) {
                    Ok(n) => n as isize,
                    Err(_) => -1,
                }
            } else {
                -1
            }
        }
        _ => -1,
    }
}
