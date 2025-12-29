//! # 扩展属性测试
//!
//! 测试文件扩展属性的设置、获取、列出和删除功能。

#![cfg(test)]

extern crate alloc;

use kernel::subsystems::fs::xattr::{
    XattrSet, XattrNamespace, XattrEntry,
    XATTR_CREATE, XATTR_REPLACE,
    XATTR_NAME_MAX, XATTR_SIZE_MAX, XATTR_LIST_MAX,
};

/// 测试扩展属性命名空间解析
#[test]
fn test_xattr_namespace_parsing() {
    // 有效的命名空间
    assert_eq!(
        XattrNamespace::from_name("user.comment").unwrap(),
        XattrNamespace::User
    );
    assert_eq!(
        XattrNamespace::from_name("trusted.key").unwrap(),
        XattrNamespace::Trusted
    );
    assert_eq!(
        XattrNamespace::from_name("security.selinux").unwrap(),
        XattrNamespace::Security
    );
    assert_eq!(
        XattrNamespace::from_name("system.data").unwrap(),
        XattrNamespace::System
    );

    // 无效的命名空间
    assert!(XattrNamespace::from_name("invalid.name").is_err());
    assert!(XattrNamespace::from_name("nosuffix").is_err());
    assert!(XattrNamespace::from_name("").is_err());
}

/// 测试扩展属性命名空间前缀
#[test]
fn test_xattr_namespace_prefix() {
    assert_eq!(XattrNamespace::User.prefix(), "user");
    assert_eq!(XattrNamespace::Trusted.prefix(), "trusted");
    assert_eq!(XattrNamespace::Security.prefix(), "security");
    assert_eq!(XattrNamespace::System.prefix(), "system");
}

/// 测试扩展属性创建
#[test]
fn test_xattr_entry_creation() {
    let entry = XattrEntry::new("user.test".to_string(), b"hello".to_vec());

    assert_eq!(entry.name, "user.test");
    assert_eq!(entry.value, b"hello");
    assert_eq!(entry.size(), 5);
    assert!(entry.created_at > 0);
    assert!(entry.modified_at > 0);
}

/// 测试设置扩展属性
#[test]
fn test_xattr_set() {
    let set = XattrSet::new(1);

    // 设置属性
    set.set("user.comment", b"This is a comment", 0).unwrap();

    // 验证属性已设置
    let names = set.get_names();
    assert_eq!(names.len(), 1);
    assert!(names.contains(&"user.comment".to_string()));
}

/// 测试获取扩展属性
#[test]
fn test_xattr_get() {
    let set = XattrSet::new(1);

    // 设置属性
    let value = b"test value";
    set.set("user.data", value, 0).unwrap();

    // 获取属性
    let mut buf = [0u8; 256];
    let size = set.get("user.data", &mut buf).unwrap();

    assert_eq!(size, value.len());
    assert_eq!(&buf[..size], value);
}

/// 测试替换扩展属性
#[test]
fn test_xattr_replace() {
    let set = XattrSet::new(1);

    // 创建属性
    set.set("user.test", b"original", 0).unwrap();

    // 替换属性
    set.set("user.test", b"replaced", XATTR_REPLACE).unwrap();

    // 验证替换
    let mut buf = [0u8; 256];
    let size = set.get("user.test", &mut buf).unwrap();
    assert_eq!(&buf[..size], b"replaced");
}

/// 测试创建模式
#[test]
fn test_xattr_create_mode() {
    let set = XattrSet::new(1);

    // 创建新属性
    set.set("user.new", b"value", XATTR_CREATE).unwrap();

    // 尝试再次创建（应该失败）
    let result = set.set("user.new", b"another", XATTR_CREATE);
    assert!(result.is_err());

    // 替换应该成功
    set.set("user.new", b"another", XATTR_REPLACE).unwrap();
}

/// 测试删除扩展属性
#[test]
fn test_xattr_remove() {
    let set = XattrSet::new(1);

    // 设置属性
    set.set("user.temp", b"temporary", 0).unwrap();

    // 删除属性
    set.remove("user.temp").unwrap();

    // 验证已删除
    let names = set.get_names();
    assert_eq!(names.len(), 0);
}

/// 测试列出扩展属性
#[test]
fn test_xattr_list() {
    let set = XattrSet::new(1);

    // 设置多个属性
    set.set("user.attr1", b"value1", 0).unwrap();
    set.set("user.attr2", b"value2", 0).unwrap();
    set.set("trusted.key", b"secret", 0).unwrap();

    // 列出属性
    let mut buf = [0u8; 1024];
    let size = set.list(&mut buf).unwrap();

    // 验证列表包含所有属性
    let list_str = core::str::from_utf8(&buf[..size]).unwrap();
    assert!(list_str.contains("user.attr1"));
    assert!(list_str.contains("user.attr2"));
    assert!(list_str.contains("trusted.key"));
}

/// 测试大小限制
#[test]
fn test_xattr_size_limits() {
    let set = XattrSet::new(1);

    // 名称长度限制
    let long_name = "a".repeat(XATTR_NAME_MAX + 1);
    let result = set.set(&long_name, b"value", 0);
    assert!(result.is_err());

    // 值大小限制
    let large_value = vec![0u8; XATTR_SIZE_MAX + 1];
    let result = set.set("user.large", &large_value, 0);
    assert!(result.is_err());

    // 正常大小应该成功
    let normal_value = vec![0u8; XATTR_SIZE_MAX];
    let result = set.set("user.normal", &normal_value, 0);
    assert!(result.is_ok());
}

/// 测试多个命名空间
#[test]
fn test_multiple_namespaces() {
    let set = XattrSet::new(1);

    // 设置不同命名空间的属性
    set.set("user.data", b"user value", 0).unwrap();
    set.set("trusted.key", b"trusted value", 0).unwrap();
    set.set("security.selinux", b"system_u:object_r:file_t:s0", 0).unwrap();
    set.set("system.info", b"system value", 0).unwrap();

    // 验证所有属性都存在
    let names = set.get_names();
    assert_eq!(names.len(), 4);
}

/// 测试空属性值
#[test]
fn test_empty_attribute_value() {
    let set = XattrSet::new(1);

    // 设置空值
    set.set("user.empty", b"", 0).unwrap();

    // 获取空值
    let mut buf = [0u8; 10];
    let size = set.get("user.empty", &mut buf).unwrap();
    assert_eq!(size, 0);
}

/// 测试属性计数
#[test]
fn test_xattr_count() {
    let set = XattrSet::new(1);

    assert_eq!(set.count(), 0);

    set.set("user.attr1", b"value1", 0).unwrap();
    assert_eq!(set.count(), 1);

    set.set("user.attr2", b"value2", 0).unwrap();
    assert_eq!(set.count(), 2);

    set.remove("user.attr1").unwrap();
    assert_eq!(set.count(), 1);
}

/// 测试属性更新时间戳
#[test]
fn test_xattr_timestamps() {
    let set = XattrSet::new(1);

    set.set("user.test", b"initial", 0).unwrap();

    let names = set.get_names();
    let name = &names[0];

    let attrs = set.attrs.lock();
    let entry = attrs.get(name).unwrap();

    let created = entry.created_at;
    let modified = entry.modified_at;

    // 等待一小段时间（模拟）
    drop(attrs);

    set.set("user.test", b"updated", XATTR_REPLACE).unwrap();

    let attrs = set.attrs.lock();
    let entry = attrs.get(name).unwrap();

    // 创建时间不应该改变
    assert_eq!(entry.created_at, created);
    // 修改时间应该更新
    assert!(entry.modified_at >= modified);
}

/// 测试特殊字符
#[test]
fn test_xattr_special_characters() {
    let set = XattrSet::new(1);

    // 测试各种特殊字符
    set.set("user.with_underscore", b"value", 0).unwrap();
    set.set("user.with-dash", b"value", 0).unwrap();
    set.set("user.with.dot", b"value", 0).unwrap();

    let names = set.get_names();
    assert_eq!(names.len(), 3);
}

/// 性能测试：大量属性
#[test]
fn test_many_attributes() {
    let set = XattrSet::new(1);

    // 创建大量属性
    for i in 0..100 {
        let name = format!("user.attr{}", i);
        set.set(&name, b"value", 0).unwrap();
    }

    assert_eq!(set.count(), 100);

    // 列出所有属性
    let mut buf = [0u8; XATTR_LIST_MAX];
    let size = set.list(&mut buf).unwrap();
    assert!(size > 0);
}

/// 边界测试：属性名称
#[test]
fn test_edge_case_names() {
    let set = XattrSet::new(1);

    // 最短有效名称
    set.set("user.a", b"value", 0).unwrap();

    // 带点的名称
    set.set("user.a.b", b"value", 0).unwrap();

    let names = set.get_names();
    assert_eq!(names.len(), 2);
}

/// 测试无效操作
#[test]
fn test_invalid_operations() {
    let set = XattrSet::new(1);

    // 获取不存在的属性
    let mut buf = [0u8; 100];
    let result = set.get("user.nonexistent", &mut buf);
    assert!(result.is_err());

    // 删除不存在的属性
    let result = set.remove("user.nonexistent");
    assert!(result.is_err());

    // 替换不存在的属性
    let result = set.set("user.nonexistent", b"value", XATTR_REPLACE);
    assert!(result.is_err());
}

/// 测试列表缓冲区大小
#[test]
fn test_list_buffer_size() {
    let set = XattrSet::new(1);

    // 设置一些属性
    set.set("user.a", b"value1", 0).unwrap();
    set.set("user.b", b"value2", 0).unwrap();

    // 缓冲区太小
    let mut small_buf = [0u8; 5];
    let result = set.list(&mut small_buf);
    assert!(result.is_err());

    // 足够大的缓冲区
    let mut large_buf = [0u8; 1024];
    let result = set.list(&mut large_buf);
    assert!(result.is_ok());
}
