// Command-line argument parsing for bootloader
//
// Features:
// - Whitespace-separated arguments
// - Key-value pairs (key=value)
// - Flag detection (--flag, -f)
// - Unknown parameter skipping (error-tolerant)
// - Dynamic memory allocation via alloc

use alloc::vec::Vec;
use alloc::string::String;

pub struct CmdLine {
    args: Vec<String>,
}

impl CmdLine {
    /// Parse command line from null-terminated string
    /// Uses single-pass state machine for efficiency
    pub fn parse(cmdline_ptr: *const u8) -> Self {
        let mut args = Vec::new();
        unsafe {
            let mut current = cmdline_ptr;
            let mut arg_start = cmdline_ptr;
            let mut in_arg = false;

            // Single-pass scanning
            while *current != 0 {
                match *current {
                    // Whitespace separators
                    b' ' | b'\t' | b'\n' | b'\r' => {
                        if in_arg {
                            let len = current as usize - arg_start as usize;
                            if len > 0 && len < 256 {
                                let arg_bytes =
                                    core::slice::from_raw_parts(arg_start, len);
                                if let Ok(s) = core::str::from_utf8(arg_bytes) {
                                    args.push(String::from(s));
                                }
                            }
                            in_arg = false;
                        }
                    }
                    _ => {
                        if !in_arg {
                            arg_start = current;
                            in_arg = true;
                        }
                    }
                }
                current = current.add(1);
            }

            // Handle last argument
            if in_arg {
                let len = current as usize - arg_start as usize;
                if len > 0 && len < 256 {
                    let arg_bytes = core::slice::from_raw_parts(arg_start, len);
                    if let Ok(s) = core::str::from_utf8(arg_bytes) {
                        args.push(String::from(s));
                    }
                }
            }
        }
        Self { args }
    }

    /// Get argument at index
    pub fn get(&self, index: usize) -> Option<&str> {
        self.args.get(index).map(|s| s.as_str())
    }

    /// Check if flag exists
    pub fn has_flag(&self, flag: &str) -> bool {
        self.args.iter().any(|arg| arg == flag)
    }

    /// Get value for option with key=value format
    pub fn get_key_value(&self, key: &str) -> Option<&str> {
        for arg in &self.args {
            if let Some(eq_pos) = arg.find('=') && &arg[..eq_pos] == key {
                return Some(&arg[eq_pos + 1..]);
            }
        }
        None
    }

    /// Get value for option (--flag value format)
    pub fn get_option(&self, opt: &str) -> Option<&str> {
        for i in 0..self.args.len() {
            if self.args[i] == opt && i + 1 < self.args.len() {
                return Some(&self.args[i + 1]);
            }
        }
        None
    }

    /// Get number of arguments
    pub fn len(&self) -> usize {
        self.args.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    /// Skip unknown parameters - returns filtered args
    /// Known parameters: flags starting with - or -- and key=value pairs
    pub fn filter_known(&self, allowed_flags: &[&str], allowed_keys: &[&str]) -> Vec<&str> {
        self.args.iter()
            .filter(|arg| {
                // Check if it's a known flag
                if arg.starts_with('-') {
                    return allowed_flags.iter().any(|f| f == arg);
                }
                // Check if it's a known key=value
                if let Some(eq_pos) = arg.find('=') {
                    let key = &arg[..eq_pos];
                    return allowed_keys.iter().any(|k| k == &key);
                }
                // Unknown parameter, skip
                false
            })
            .map(|s| s.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmdline_parsing() {
        let cmdline = "kernel arg1 arg2";
        let parsed = CmdLine::parse(cmdline.as_ptr());
        assert_eq!(parsed.len(), 3);
    }

    #[test]
    fn test_cmdline_has_flag() {
        let cmdline = "--verbose --debug";
        let parsed = CmdLine::parse(cmdline.as_ptr());
        assert!(parsed.has_flag("--verbose"));
    }
}
