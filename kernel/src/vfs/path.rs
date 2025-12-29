//! 路径处理模块
//!
//! 提供路径解析、规范化和操作功能。

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// 文件系统路径
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path {
    /// 路径字符串
    inner: String,
}

impl Path {
    /// 创建新的路径
    pub fn new<S: AsRef<str>>(path: S) -> Self {
        Self {
            inner: path.as_ref().to_string(),
        }
    }

    /// 获取路径字符串
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// 判断是否是绝对路径
    pub fn is_absolute(&self) -> bool {
        self.inner.starts_with('/')
    }

    /// 获取父目录
    pub fn parent(&self) -> Option<Self> {
        if self.inner == "/" {
            return None;
        }

        // 移除末尾的斜杠
        let path = self.inner.trim_end_matches('/');

        // 查找最后一个斜杠
        if let Some(pos) = path.rfind('/') {
            if pos == 0 {
                Some(Path::new("/"))
            } else {
                Some(Path::new(&path[..pos]))
            }
        } else {
            None
        }
    }

    /// 获取文件名
    pub fn file_name(&self) -> Option<&str> {
        let path = self.inner.trim_end_matches('/');

        if path == "/" {
            return None;
        }

        // 查找最后一个斜杠
        if let Some(pos) = path.rfind('/') {
            if pos + 1 < path.len() {
                Some(&path[pos + 1..])
            } else {
                None
            }
        } else {
            Some(path)
        }
    }

    /// 连接路径
    pub fn join(&self, other: &Path) -> Path {
        if other.is_absolute() {
            return other.clone();
        }

        let mut result = self.inner.clone();

        // 确保路径以斜杠结尾
        if !result.ends_with('/') {
            result.push('/');
        }

        result.push_str(other.as_str());
        Path::new(result)
    }

    /// 推送路径组件
    pub fn push(&mut self, component: &Path) {
        if component.is_absolute() {
            self.inner = component.inner.clone();
        } else {
            if !self.inner.ends_with('/') && !self.inner.is_empty() {
                self.inner.push('/');
            }
            self.inner.push_str(component.as_str());
        }
    }

    /// 规范化路径
    pub fn canonicalize(&self) -> Path {
        let mut components = Vec::new();
        let parts: Vec<&str> = self.inner.split('/').filter(|s| !s.is_empty()).collect();

        for part in parts {
            match part {
                "." => continue,
                ".." => {
                    if !components.is_empty() {
                        components.pop();
                    }
                }
                _ => components.push(part),
            }
        }

        if components.is_empty() {
            return Path::new("/");
        }

        let result = "/".to_string() + &components.join("/");
        Path::new(result)
    }

    /// 转换为字符串
    pub fn to_string(&self) -> String {
        self.inner.clone()
    }

    /// 转换为拥有的字符串
    pub fn into_string(self) -> String {
        self.inner
    }

    /// 判断路径是否为空
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// 判断是否是根路径
    pub fn is_root(&self) -> bool {
        self.inner == "/"
    }
}

impl AsRef<str> for Path {
    fn as_ref(&self) -> &str {
        &self.inner
    }
}

impl From<&str> for Path {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Path {
    fn from(s: String) -> Self {
        Self { inner: s }
    }
}

impl core::fmt::Display for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_absolute_path() {
        let path = Path::new("/usr/bin/bash");
        assert!(path.is_absolute());
        assert!(!Path::new("relative/path").is_absolute());
    }

    #[test]
    fn test_parent() {
        let path = Path::new("/usr/bin/bash");
        assert_eq!(path.parent().unwrap().as_str(), "/usr/bin");

        let root = Path::new("/");
        assert!(root.parent().is_none());
    }

    #[test]
    fn test_file_name() {
        let path = Path::new("/usr/bin/bash");
        assert_eq!(path.file_name(), Some("bash"));

        let root = Path::new("/");
        assert!(root.file_name().is_none());
    }

    #[test]
    fn test_join() {
        let path = Path::new("/usr/bin");
        let result = path.join(&Path::new("bash"));
        assert_eq!(result.as_str(), "/usr/bin/bash");
    }

    #[test]
    fn test_canonicalize() {
        let path = Path::new("/usr/../usr/bin/./bash");
        let result = path.canonicalize();
        assert_eq!(result.as_str(), "/usr/bin/bash");
    }
}
