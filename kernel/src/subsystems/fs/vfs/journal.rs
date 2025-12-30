//! VFS 基础日志/写序列化策略骨架
//! 当前仅提供接口占位，未真正持久化。

#[derive(Debug, Clone)]
pub struct JournalOptions {
    pub enabled: bool,
    pub sync_on_commit: bool,
}

impl Default for JournalOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            sync_on_commit: false,
        }
    }
}

impl JournalOptions {
    pub const fn default() -> Self {
        Self {
            enabled: false,
            sync_on_commit: false,
        }
    }
}

#[derive(Debug)]
pub enum JournalError {
    Disabled,
    Io,
}

#[derive(Debug)]
pub struct Journal {
    pub opts: JournalOptions,
}

impl Journal {
    pub const fn new(opts: JournalOptions) -> Self {
        Self { opts }
    }
    
    pub fn record(&self, _entry: &str) -> Result<(), JournalError> {
        if !self.opts.enabled {
            return Err(JournalError::Disabled);
        }
        // 占位：真实实现需写入日志缓冲
        Ok(())
    }

    pub fn flush(&self) -> Result<(), JournalError> {
        if !self.opts.enabled {
            return Err(JournalError::Disabled);
        }
        // 占位：同步到持久层
        Ok(())
    }
}

