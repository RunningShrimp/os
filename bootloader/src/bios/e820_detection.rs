/// E820 Memory Detection via BIOS INT 0x15
///
/// Detects and enumerates the system memory map using the E820h interface.
/// This is the standard way to discover available memory in x86 systems.

use crate::bios::bios_realmode::{int15_e820, RealModeExecutor};

/// Maximum number of E820 entries to track
const MAX_E820_ENTRIES: usize = 32;

/// E820 memory entry
#[derive(Debug, Clone, Copy)]
pub struct E820Entry {
    pub base_address: u64,
    pub length: u64,
    pub entry_type: u32,
}

impl E820Entry {
    /// Check if this entry is usable RAM
    pub fn is_usable(&self) -> bool {
        self.entry_type == 1 // Type 1 = Usable (RAM)
    }

    /// Get human-readable type name
    pub fn type_name(&self) -> &'static str {
        match self.entry_type {
            1 => "Usable RAM",
            2 => "Reserved",
            3 => "ACPI Reclaimable",
            4 => "ACPI NVS",
            5 => "Bad Memory",
            _ => "Unknown",
        }
    }
}

/// E820 memory map result
pub struct E820MemoryMap {
    pub entries: [Option<E820Entry>; MAX_E820_ENTRIES],
    pub count: usize,
}

impl E820MemoryMap {
    /// Create empty memory map
    pub fn new() -> Self {
        Self {
            entries: [None; MAX_E820_ENTRIES],
            count: 0,
        }
    }

    /// Add entry to memory map
    pub fn add_entry(&mut self, entry: E820Entry) -> bool {
        if self.count >= MAX_E820_ENTRIES {
            return false; // Map is full
        }
        self.entries[self.count] = Some(entry);
        self.count += 1;
        true
    }

    /// Get total usable RAM in bytes
    pub fn total_ram(&self) -> u64 {
        self.entries
            .iter()
            .filter_map(|e| e.as_ref().filter(|entry| entry.is_usable()))
            .map(|e| e.length)
            .sum()
    }

    /// Find the highest usable address
    pub fn highest_usable_address(&self) -> u64 {
        self.entries
            .iter()
            .filter_map(|e| {
                e.as_ref()
                    .filter(|entry| entry.is_usable())
                    .map(|entry| entry.base_address + entry.length)
            })
            .max()
            .unwrap_or(0)
    }

    /// Check if an address range is usable
    pub fn is_range_usable(&self, base: u64, length: u64) -> bool {
        let end = base + length;
        self.entries
            .iter()
            .filter_map(|e| e.as_ref().filter(|entry| entry.is_usable()))
            .any(|entry| {
                let entry_end = entry.base_address + entry.length;
                // Check if ranges overlap and entire range is within entry
                base >= entry.base_address && end <= entry_end
            })
    }
}

/// Detect system memory using E820 BIOS interface
///
/// Calls BIOS INT 0x15/AX=0xE820 repeatedly to enumerate all memory regions.
/// Requires real mode execution capability.
pub fn detect_e820_memory(
    executor: &RealModeExecutor,
    buffer_addr: u32,
) -> Result<E820MemoryMap, &'static str> {
    let mut map = E820MemoryMap::new();
    let mut continuation = 0u32;

    loop {
        // Call INT 0x15/E820 to get next entry
        let (bytes_written, next_continuation) =
            int15_e820::call_e820(executor, buffer_addr, continuation)
                .map_err(|_| "E820 detection failed")?;

        // Check if we got data
        if bytes_written == 0 {
            break; // No more entries
        }

        // Parse entry from buffer (in low memory, need to read from address)
        // SAFETY: Caller must ensure buffer_addr points to valid low memory with >= 24 bytes
        let entry = unsafe {
            let ptr = buffer_addr as *const E820RawEntry;
            E820RawEntry::from_ptr(ptr)
        };

        // Add to map
        let e820_entry = E820Entry {
            base_address: entry.base_address,
            length: entry.length,
            entry_type: entry.entry_type,
        };

        if !map.add_entry(e820_entry) {
            break; // Map is full
        }

        // Check for end of list
        if next_continuation == 0 {
            break; // No more entries
        }

        continuation = next_continuation;
    }

    if map.count == 0 {
        return Err("No E820 entries detected");
    }

    Ok(map)
}

/// Raw E820 entry structure (as returned by BIOS)
#[repr(C, packed)]
struct E820RawEntry {
    pub base_address: u64,
    pub length: u64,
    pub entry_type: u32,
    pub extended_attributes: u32, // ACPI 3.0+
}

impl E820RawEntry {
    /// Parse entry from pointer
    unsafe fn from_ptr(ptr: *const Self) -> Self {
        core::ptr::read(ptr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e820_entry_creation() {
        let entry = E820Entry {
            base_address: 0x0,
            length: 0x100000,
            entry_type: 1,
        };

        assert_eq!(entry.base_address, 0x0);
        assert_eq!(entry.length, 0x100000);
        assert!(entry.is_usable());
    }

    #[test]
    fn test_e820_entry_types() {
        let usable = E820Entry {
            base_address: 0,
            length: 0x1000,
            entry_type: 1,
        };
        assert!(usable.is_usable());
        assert_eq!(usable.type_name(), "Usable RAM");

        let reserved = E820Entry {
            base_address: 0xF0000,
            length: 0x10000,
            entry_type: 2,
        };
        assert!(!reserved.is_usable());
        assert_eq!(reserved.type_name(), "Reserved");
    }

    #[test]
    fn test_memory_map_creation() {
        let map = E820MemoryMap::new();
        assert_eq!(map.count, 0);
        assert_eq!(map.total_ram(), 0);
    }

    #[test]
    fn test_memory_map_add_entry() {
        let mut map = E820MemoryMap::new();
        let entry = E820Entry {
            base_address: 0x0,
            length: 0x100000,
            entry_type: 1,
        };

        assert!(map.add_entry(entry));
        assert_eq!(map.count, 1);
        assert_eq!(map.total_ram(), 0x100000);
    }

    #[test]
    fn test_memory_map_full() {
        let mut map = E820MemoryMap::new();
        let entry = E820Entry {
            base_address: 0x0,
            length: 0x1000,
            entry_type: 1,
        };

        // Fill the map
        for _ in 0..MAX_E820_ENTRIES {
            assert!(map.add_entry(entry));
        }

        // Try to add one more (should fail)
        assert!(!map.add_entry(entry));
    }

    #[test]
    fn test_highest_usable_address() {
        let mut map = E820MemoryMap::new();

        let entry1 = E820Entry {
            base_address: 0x0,
            length: 0x10000,
            entry_type: 1,
        };
        map.add_entry(entry1);

        let entry2 = E820Entry {
            base_address: 0x100000,
            length: 0x20000,
            entry_type: 1,
        };
        map.add_entry(entry2);

        assert_eq!(map.highest_usable_address(), 0x120000);
    }

    #[test]
    fn test_is_range_usable() {
        let mut map = E820MemoryMap::new();
        map.add_entry(E820Entry {
            base_address: 0x0,
            length: 0x100000,
            entry_type: 1,
        });

        // Range within usable area
        assert!(map.is_range_usable(0x0, 0x10000));
        assert!(map.is_range_usable(0x50000, 0x20000));

        // Range outside usable area
        assert!(!map.is_range_usable(0x100000, 0x10000));

        // Range partially outside
        assert!(!map.is_range_usable(0xF0000, 0x20000));
    }

    #[test]
    fn test_multiple_entry_types() {
        let mut map = E820MemoryMap::new();

        // Add various entry types
        map.add_entry(E820Entry {
            base_address: 0x0,
            length: 0x100000,
            entry_type: 1, // Usable
        });

        map.add_entry(E820Entry {
            base_address: 0xF0000,
            length: 0x10000,
            entry_type: 2, // Reserved
        });

        map.add_entry(E820Entry {
            base_address: 0x100000,
            length: 0x100000,
            entry_type: 1, // Usable
        });

        assert_eq!(map.count, 3);
        assert_eq!(map.total_ram(), 0x200000); // Only count type 1
    }
}
