    /// Allocate a new process
    pub fn alloc(&mut self) -> Option<&mut Proc> {
        for (i, proc) in self.procs.iter_mut().enumerate() {
            if proc.state == ProcState::Unused {
                let new_pid = self.next_pid;
                proc.pid = new_pid;
                self.next_pid += 1;
                proc.state = ProcState::Used;
                
                // Initialize signal state
                proc.signals = Some(SignalState::new());
                
                // Allocate kernel stack
                let kstack = kalloc();
                if kstack.is_null() {
                    proc.state = ProcState::Unused;
                    proc.signals = None;
                    return None;
                }
                proc.kstack = kstack as usize + PAGE_SIZE;  // Stack grows down
                
                // Allocate trapframe page
                let tf = kalloc();
                if tf.is_null() {
                    unsafe { kfree(kstack); }
                    proc.state = ProcState::Unused;
                    proc.signals = None;
                    return None;
                }
                proc.trapframe = tf as *mut TrapFrame;
                
                // Add to pid_to_index map for O(1) lookups
                self.pid_to_index.insert(new_pid, i);
                
                return Some(proc);
            }
        }
        None
    }