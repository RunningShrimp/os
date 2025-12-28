//! FD 语义校验占位：close-on-exec/dup*/fcntl 标志

#[derive(Debug, Default, Clone, Copy)]
pub struct FdFlags {
    pub close_on_exec: bool,
    pub cloexec_supported: bool,
}

impl FdFlags {
    pub fn merge_dup(&self, parent: &FdFlags) -> Self {
        // dup/dup2/dup3 通常保留 cloexec=0，dup3 可指定
        let mut f = *self;
        f.close_on_exec = false;
        f.cloexec_supported = parent.cloexec_supported;
        f
    }
}

