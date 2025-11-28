//! File types and modes for VFS

/// File type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
    Symlink,
}

/// File mode/permissions
#[derive(Debug, Clone, Copy, Default)]
pub struct FileMode(pub u32);

impl FileMode {
    pub const S_IFMT: u32   = 0o170000;  // Type mask
    pub const S_IFREG: u32  = 0o100000;  // Regular file
    pub const S_IFDIR: u32  = 0o040000;  // Directory
    pub const S_IFCHR: u32  = 0o020000;  // Character device
    pub const S_IFBLK: u32  = 0o060000;  // Block device
    pub const S_IFIFO: u32  = 0o010000;  // FIFO
    pub const S_IFSOCK: u32 = 0o140000;  // Socket
    pub const S_IFLNK: u32  = 0o120000;  // Symbolic link
    
    pub const S_ISUID: u32  = 0o4000;   // Set UID
    pub const S_ISGID: u32  = 0o2000;   // Set GID
    pub const S_ISVTX: u32  = 0o1000;   // Sticky bit
    
    pub const S_IRWXU: u32  = 0o700;    // User RWX
    pub const S_IRUSR: u32  = 0o400;    // User read
    pub const S_IWUSR: u32  = 0o200;    // User write
    pub const S_IXUSR: u32  = 0o100;    // User execute
    
    pub const S_IRWXG: u32  = 0o070;    // Group RWX
    pub const S_IRGRP: u32  = 0o040;    // Group read
    pub const S_IWGRP: u32  = 0o020;    // Group write
    pub const S_IXGRP: u32  = 0o010;    // Group execute
    
    pub const S_IRWXO: u32  = 0o007;    // Other RWX
    pub const S_IROTH: u32  = 0o004;    // Other read
    pub const S_IWOTH: u32  = 0o002;    // Other write
    pub const S_IXOTH: u32  = 0o001;    // Other execute
    
    pub fn new(mode: u32) -> Self {
        Self(mode)
    }
    
    pub fn file_type(&self) -> FileType {
        match self.0 & Self::S_IFMT {
            Self::S_IFREG => FileType::Regular,
            Self::S_IFDIR => FileType::Directory,
            Self::S_IFCHR => FileType::CharDevice,
            Self::S_IFBLK => FileType::BlockDevice,
            Self::S_IFIFO => FileType::Fifo,
            Self::S_IFSOCK => FileType::Socket,
            Self::S_IFLNK => FileType::Symlink,
            _ => FileType::Regular,
        }
    }
    
    pub fn is_dir(&self) -> bool {
        self.0 & Self::S_IFMT == Self::S_IFDIR
    }
    
    pub fn is_regular(&self) -> bool {
        self.0 & Self::S_IFMT == Self::S_IFREG
    }
    
    pub fn permissions(&self) -> u32 {
        self.0 & 0o777
    }
}

/// File attributes (stat structure equivalent)
#[derive(Debug, Clone, Default)]
pub struct FileAttr {
    pub ino: u64,           // Inode number
    pub mode: FileMode,     // Mode and permissions
    pub nlink: u32,         // Number of hard links
    pub uid: u32,           // Owner user ID
    pub gid: u32,           // Owner group ID
    pub size: u64,          // Size in bytes
    pub blksize: u32,       // Block size
    pub blocks: u64,        // Number of 512B blocks
    pub atime: u64,         // Access time
    pub mtime: u64,         // Modification time
    pub ctime: u64,         // Change time
    pub rdev: u64,          // Device ID (for device files)
}

/// Seek whence
#[derive(Debug, Clone, Copy)]
pub enum SeekWhence {
    Set,  // Absolute position
    Cur,  // Relative to current
    End,  // Relative to end
}