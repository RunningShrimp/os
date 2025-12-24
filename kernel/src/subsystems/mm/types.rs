/// Memory type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryType {
    Normal = 0,
    DMA = 1,
    HighMem = 2,
    Reserved = 3,
    Kernel = 4,
    User = 5,
}

impl Default for MemoryType {
    fn default() -> Self {
        MemoryType::Normal
    }
}
