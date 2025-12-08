// 调试符号支持模块

extern crate alloc;
//
// 提供全面的调试符号支持，包括符号表管理、符号解析、
// 地址反向查找和调试信息生成。
//
// 主要功能：
// - 符号表管理
// - 符号解析和查找
// - 地址到符号的反向查找
// - 调试信息生成
// - 源代码级调试支持
// - DWARF调试信息解析
// - 符号缓存优化

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

// Import println macro
#[allow(unused_imports)]
use crate::println;

/// 符号类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    /// 函数符号
    Function,
    /// 变量符号
    Variable,
    /// 类型符号
    Type,
    /// 常量符号
    Constant,
    /// 标签符号
    Label,
    /// 未知符号
    Unknown,
}

/// 符号绑定类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolBinding {
    /// 局部符号
    Local,
    /// 全局符号
    Global,
    /// 弱符号
    Weak,
    /// 未知绑定
    Unknown,
}

/// 符号可见性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolVisibility {
    /// 默认可见性
    Default,
    /// 内部可见性
    Internal,
    /// 隐藏可见性
    Hidden,
    /// 未知可见性
    Unknown,
}

/// 调试符号
#[derive(Debug, Clone)]
pub struct DebugSymbol {
    /// 符号名称
    pub name: String,
    /// 符号地址
    pub address: usize,
    /// 符号大小
    pub size: usize,
    /// 符号类型
    pub symbol_type: SymbolType,
    /// 符号绑定
    pub binding: SymbolBinding,
    /// 符号可见性
    pub visibility: SymbolVisibility,
    /// 所属模块
    pub module: Option<String>,
    /// 源文件
    pub source_file: Option<String>,
    /// 行号
    pub line_number: Option<u32>,
    /// 列号
    pub column_number: Option<u32>,
    /// 附加信息
    pub additional_info: BTreeMap<String, String>,
}

/// 源代码位置信息
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// 文件路径
    pub file_path: String,
    /// 行号
    pub line: u32,
    /// 列号
    pub column: u32,
    /// 函数名
    pub function_name: Option<String>,
    /// 行内容
    pub line_content: Option<String>,
}

/// 行号映射表项
#[derive(Debug, Clone)]
pub struct LineMapping {
    /// 虚拟地址
    pub address: usize,
    /// 源文件路径
    pub file_path: String,
    /// 行号
    pub line_number: u32,
    /// 列号
    pub column_number: u32,
}

/// 函数范围信息
#[derive(Debug, Clone)]
pub struct FunctionRange {
    /// 函数名
    pub name: String,
    /// 起始地址
    pub start_address: usize,
    /// 结束地址
    pub end_address: usize,
    /// 入口点地址
    pub entry_address: usize,
    /// 源文件路径
    pub source_file: Option<String>,
    /// 起始行号
    pub start_line: Option<u32>,
    /// 结束行号
    pub end_line: Option<u32>,
}

/// 类型信息
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// 类型名称
    pub name: String,
    /// 类型大小
    pub size: usize,
    /// 类型对齐
    pub alignment: usize,
    /// 类型种类
    pub kind: TypeKind,
    /// 字段信息
    pub fields: Vec<FieldInfo>,
    /// 方法信息
    pub methods: Vec<MethodInfo>,
}

/// 类型种类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    /// 基本类型
    Primitive,
    /// 结构体
    Struct,
    /// 联合体
    Union,
    /// 枚举
    Enum,
    /// 数组
    Array,
    /// 指针
    Pointer,
    /// 函数
    Function,
    /// 未知类型
    Unknown,
}

/// 字段信息
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// 字段名
    pub name: String,
    /// 字段类型
    pub field_type: String,
    /// 字段偏移
    pub offset: usize,
    /// 字段大小
    pub size: usize,
    /// 位字段信息
    pub bit_field: Option<BitFieldInfo>,
}

/// 位字段信息
#[derive(Debug, Clone)]
pub struct BitFieldInfo {
    /// 起始位
    pub start_bit: u8,
    /// 位长度
    pub bit_length: u8,
}

/// 方法信息
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// 方法名
    pub name: String,
    /// 返回类型
    pub return_type: String,
    /// 参数类型
    pub parameter_types: Vec<String>,
    /// 是否为虚方法
    pub is_virtual: bool,
    /// 是否为静态方法
    pub is_static: bool,
    /// 访问修饰符
    pub access_modifier: AccessModifier,
}

/// 访问修饰符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessModifier {
    /// 公有
    Public,
    /// 私有
    Private,
    /// 保护
    Protected,
    /// 默认
    Default,
}

/// DWARF调试信息
#[derive(Debug, Clone)]
pub struct DwarfDebugInfo {
    /// 编译单元
    pub compilation_units: Vec<CompilationUnit>,
    /// 调试节
    pub debug_sections: DebugSections,
}

/// 编译单元
#[derive(Debug, Clone)]
pub struct CompilationUnit {
    /// 单元名称
    pub name: String,
    /// 编译目录
    pub compilation_directory: String,
    /// 语言
    pub language: DebugLanguage,
    /// 调试信息条目
    pub debug_entries: Vec<DebugEntry>,
    /// 行号程序
    pub line_program: Option<LineProgram>,
}

/// 调试语言
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugLanguage {
    /// C语言
    C,
    /// C++语言
    Cpp,
    /// Rust语言
    Rust,
    /// 汇编语言
    Assembly,
    /// 未知语言
    Unknown,
}

/// 调试信息条目
#[derive(Debug, Clone)]
pub struct DebugEntry {
    /// 条目标签
    pub tag: DebugTag,
    /// 是否有子条目
    pub has_children: bool,
    /// 属性
    pub attributes: BTreeMap<DebugAttribute, DebugAttributeValue>,
}

/// 调试标签
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugTag {
    /// 编译单元
    CompileUnit,
    /// 子程序
    Subprogram,
    /// 变量
    Variable,
    /// 基本类型
    BaseType,
    /// 结构体类型
    StructureType,
    /// 联合体类型
    UnionType,
    /// 数组类型
    ArrayType,
    /// 指针类型
    PointerType,
    /// 枚举类型
    EnumerationType,
    /// 未知标签
    Unknown,
}

/// 调试属性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugAttribute {
    /// 名称
    Name,
    /// 地址
    Location,
    /// 大小
    ByteSize,
    /// 文件名
    DeclFile,
    /// 行号
    DeclLine,
    /// 类型
    Type,
    /// 外部链接
    External,
    /// 未知属性
    Unknown,
}

/// 调试属性值
#[derive(Debug, Clone)]
pub enum DebugAttributeValue {
    /// 字符串值
    String(String),
    /// 无符号整数
    Unsigned(u64),
    /// 有符号整数
    Signed(i64),
    /// 地址
    Address(usize),
    /// 标志
    Flag(bool),
    /// 引用
    Reference(DebugReference),
    /// 未知值
    Unknown,
}

/// 调试引用
#[derive(Debug, Clone)]
pub struct DebugReference {
    /// 引用类型
    pub reference_type: DebugReferenceType,
    /// 引用值
    pub value: u64,
}

/// 调试引用类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugReferenceType {
    /// 调试信息偏移
    DebugInfoOffset,
    /// 字符串偏移
    StringOffset,
    /// 宏信息偏移
    MacroOffset,
    /// 未知引用
    Unknown,
}

/// 行号程序
#[derive(Debug, Clone)]
pub struct LineProgram {
    /// 文件名表
    pub file_names: Vec<String>,
    /// 行号条目
    pub line_entries: Vec<LineEntry>,
}

/// 行号条目
#[derive(Debug, Clone)]
pub struct LineEntry {
    /// 地址
    pub address: usize,
    /// 文件索引
    pub file_index: u32,
    /// 行号
    pub line_number: u32,
    /// 列号
    pub column_number: u32,
    /// 是否为语句开始
    pub is_statement: bool,
    /// 基本块边界
    pub basic_block: bool,
    /// 序列结尾
    pub end_sequence: bool,
}

/// 调试节
#[derive(Debug, Clone)]
pub struct DebugSections {
    /// 调试信息节
    pub debug_info: Vec<u8>,
    /// 调试缩写节
    pub debug_abbrev: Vec<u8>,
    /// 调试行号节
    pub debug_line: Vec<u8>,
    /// 调试字符串节
    pub debug_str: Vec<u8>,
    /// 调试范围节
    pub debug_ranges: Vec<u8>,
    /// 调试帧节
    pub debug_frame: Vec<u8>,
}

/// 符号表管理器
pub struct SymbolTableManager {
    /// 全局符号表
    global_symbols: Arc<Mutex<BTreeMap<String, DebugSymbol>>>,
    /// 地址到符号的映射
    address_to_symbol: Arc<Mutex<BTreeMap<usize, String>>>,
    /// 函数范围映射
    function_ranges: Arc<Mutex<BTreeMap<usize, FunctionRange>>>,
    /// 行号映射
    line_mappings: Arc<Mutex<Vec<LineMapping>>>,
    /// 类型信息
    type_info: Arc<Mutex<BTreeMap<String, TypeInfo>>>,
    /// DWARF调试信息
    dwarf_info: Arc<Mutex<Option<DwarfDebugInfo>>>,
    /// 模块符号表
    module_symbols: Arc<Mutex<BTreeMap<String, Vec<DebugSymbol>>>>,
    /// 符号缓存
    symbol_cache: Arc<Mutex<BTreeMap<String, DebugSymbol>>>,
    /// 统计信息
    statistics: SymbolStatistics,
}

/// 符号查找结果
#[derive(Debug, Clone)]
pub struct SymbolLookupResult {
    /// 找到的符号
    pub symbol: Option<DebugSymbol>,
    /// 精确匹配
    pub exact_match: bool,
    /// 相似符号（模糊匹配）
    pub similar_symbols: Vec<DebugSymbol>,
    /// 查找耗时（纳秒）
    pub lookup_time: u64,
}

/// 反向查找结果
#[derive(Debug, Clone)]
pub struct ReverseLookupResult {
    /// 找到的符号
    pub symbol: Option<DebugSymbol>,
    /// 偏移量
    pub offset: Option<usize>,
    /// 源代码位置
    pub source_location: Option<SourceLocation>,
    /// 包含的函数
    pub containing_function: Option<FunctionRange>,
    /// 查找耗时（纳秒）
    pub lookup_time: u64,
}

/// 符号统计信息
#[derive(Debug, Default)]
pub struct SymbolStatistics {
    /// 总符号数量
    pub total_symbols: AtomicUsize,
    /// 函数符号数量
    pub function_symbols: AtomicUsize,
    /// 变量符号数量
    pub variable_symbols: AtomicUsize,
    /// 类型符号数量
    pub type_symbols: AtomicUsize,
    /// 缓存命中数
    pub cache_hits: AtomicU64,
    /// 缓存未命中数
    pub cache_misses: AtomicU64,
    /// 查找次数
    pub lookups: AtomicU64,
    /// 总查找时间（纳秒）
    pub total_lookup_time: AtomicU64,
}

/// 符号表生成器
pub struct SymbolTableGenerator {
    /// 生成选项
    options: SymbolGenerationOptions,
}

/// 符号生成选项
#[derive(Debug, Clone)]
pub struct SymbolGenerationOptions {
    /// 是否生成DWARF调试信息
    pub generate_dwarf: bool,
    /// 是否包含行号信息
    pub include_line_numbers: bool,
    /// 是否包含类型信息
    pub include_type_info: bool,
    /// 是否包含源代码位置
    pub include_source_locations: bool,
    /// 是否优化符号表大小
    pub optimize_for_size: bool,
    /// 压缩级别
    pub compression_level: u8,
}

impl Default for SymbolGenerationOptions {
    fn default() -> Self {
        Self {
            generate_dwarf: true,
            include_line_numbers: true,
            include_type_info: true,
            include_source_locations: true,
            optimize_for_size: false,
            compression_level: 0,
        }
    }
}

impl SymbolTableManager {
    /// 创建新的符号表管理器
    pub fn new() -> Self {
        Self {
            global_symbols: Arc::new(Mutex::new(BTreeMap::new())),
            address_to_symbol: Arc::new(Mutex::new(BTreeMap::new())),
            function_ranges: Arc::new(Mutex::new(BTreeMap::new())),
            line_mappings: Arc::new(Mutex::new(Vec::new())),
            type_info: Arc::new(Mutex::new(BTreeMap::new())),
            dwarf_info: Arc::new(Mutex::new(None)),
            module_symbols: Arc::new(Mutex::new(BTreeMap::new())),
            symbol_cache: Arc::new(Mutex::new(BTreeMap::new())),
            statistics: SymbolStatistics::default(),
        }
    }

    /// 添加符号
    pub fn add_symbol(&self, symbol: DebugSymbol) -> Result<(), SymbolError> {
        let start_time = crate::time::timestamp_nanos();

        // 检查符号是否已存在
        let mut global_symbols = self.global_symbols.lock();
        if global_symbols.contains_key(&symbol.name) {
            return Err(SymbolError::SymbolAlreadyExists(symbol.name));
        }

        // 添加到全局符号表
        global_symbols.insert(symbol.name.clone(), symbol.clone());

        // 更新地址到符号的映射
        let mut address_map = self.address_to_symbol.lock();
        address_map.insert(symbol.address, symbol.name.clone());

        // 如果是函数符号，添加到函数范围映射
        if symbol.symbol_type == SymbolType::Function {
            let mut ranges = self.function_ranges.lock();
            ranges.insert(symbol.address, FunctionRange {
                name: symbol.name.clone(),
                start_address: symbol.address,
                end_address: symbol.address + symbol.size,
                entry_address: symbol.address,
                source_file: symbol.source_file.clone(),
                start_line: symbol.line_number,
                end_line: None,
            });
        }

        // 更新统计
        match symbol.symbol_type {
            SymbolType::Function => self.statistics.function_symbols.fetch_add(1, Ordering::SeqCst),
            SymbolType::Variable => self.statistics.variable_symbols.fetch_add(1, Ordering::SeqCst),
            SymbolType::Type => self.statistics.type_symbols.fetch_add(1, Ordering::SeqCst),
            _ => 0,
        };
        self.statistics.total_symbols.fetch_add(1, Ordering::SeqCst);

        let lookup_time = crate::time::timestamp_nanos() - start_time;
        self.statistics.total_lookup_time.fetch_add(lookup_time, Ordering::SeqCst);

        Ok(())
    }

    /// 查找符号
    pub fn lookup_symbol(&self, name: &str) -> Result<SymbolLookupResult, SymbolError> {
        let start_time = crate::time::timestamp_nanos();

        // 首先检查缓存
        {
            let cache = self.symbol_cache.lock();
            if let Some(cached_symbol) = cache.get(name) {
                self.statistics.cache_hits.fetch_add(1, Ordering::SeqCst);
                let lookup_time = crate::time::timestamp_nanos() - start_time;
                return Ok(SymbolLookupResult {
                    symbol: Some(cached_symbol.clone()),
                    exact_match: true,
                    similar_symbols: Vec::new(),
                    lookup_time,
                });
            }
        }

        self.statistics.cache_misses.fetch_add(1, Ordering::SeqCst);

        // 在全局符号表中查找
        let global_symbols = self.global_symbols.lock();
        let symbol = global_symbols.get(name).cloned();

        let exact_match = symbol.is_some();
        let similar_symbols = if exact_match {
            Vec::new()
        } else {
            // 模糊匹配相似的符号名
            self.find_similar_symbols(name, &global_symbols)
        };

        // 如果找到符号，添加到缓存
        if let Some(ref sym) = symbol {
            let mut cache = self.symbol_cache.lock();
            cache.insert(name.to_string(), sym.clone());
        }

        self.statistics.lookups.fetch_add(1, Ordering::SeqCst);
        let lookup_time = crate::time::timestamp_nanos() - start_time;
        self.statistics.total_lookup_time.fetch_add(lookup_time, Ordering::SeqCst);

        Ok(SymbolLookupResult {
            symbol,
            exact_match,
            similar_symbols,
            lookup_time,
        })
    }

    /// 反向查找（地址到符号）
    pub fn reverse_lookup(&self, address: usize) -> Result<ReverseLookupResult, SymbolError> {
        let start_time = crate::time::timestamp_nanos();

        // 在地址到符号的映射中查找
        let address_map = self.address_to_symbol.lock();
        let symbol_name = address_map.range(..=address).rev().next().map(|(_, name)| name.clone());

        let symbol = if let Some(name) = &symbol_name {
            let global_symbols = self.global_symbols.lock();
            global_symbols.get(name).cloned()
        } else {
            None
        };

        // 计算偏移量
        let offset = symbol.as_ref().map(|s| address.saturating_sub(s.address));

        // 查找源代码位置
        let source_location = self.find_source_location(address);

        // 查找包含的函数
        let containing_function = self.find_containing_function(address);

        let lookup_time = crate::time::timestamp_nanos() - start_time;

        Ok(ReverseLookupResult {
            symbol,
            offset,
            source_location,
            containing_function,
            lookup_time,
        })
    }

    /// 添加模块符号
    pub fn add_module_symbols(&self, module_name: String, symbols: Vec<DebugSymbol>) -> Result<(), SymbolError> {
        let mut module_symbols = self.module_symbols.lock();

        for symbol in symbols {
            // 添加到模块符号表
            let module_list = module_symbols.entry(module_name.clone()).or_insert_with(Vec::new);
            module_list.push(symbol.clone());

            // 同时添加到全局符号表（如果还没有）
            let mut global_symbols = self.global_symbols.lock();
            if !global_symbols.contains_key(&symbol.name) {
                global_symbols.insert(symbol.name.clone(), symbol.clone());

                // 更新地址映射
                let mut address_map = self.address_to_symbol.lock();
                address_map.insert(symbol.address, symbol.name.clone());
            }
        }

        crate::println!("[symbols] 添加模块 {} 的 {} 个符号", module_name, module_symbols.get(&module_name).map(|s| s.len()).unwrap_or(0));

        Ok(())
    }

    /// 获取模块符号
    pub fn get_module_symbols(&self, module_name: &str) -> Result<Vec<DebugSymbol>, SymbolError> {
        let module_symbols = self.module_symbols.lock();
        Ok(module_symbols.get(module_name).cloned().unwrap_or_default())
    }

    /// 添加行号映射
    pub fn add_line_mapping(&self, mapping: LineMapping) -> Result<(), SymbolError> {
        let mut mappings = self.line_mappings.lock();
        mappings.push(mapping);
        Ok(())
    }

    /// 添加类型信息
    pub fn add_type_info(&self, type_info: TypeInfo) -> Result<(), SymbolError> {
        let mut type_map = self.type_info.lock();
        type_map.insert(type_info.name.clone(), type_info);
        Ok(())
    }

    /// 设置DWARF调试信息
    pub fn set_dwarf_info(&self, dwarf_info: DwarfDebugInfo) -> Result<(), SymbolError> {
        let mut info = self.dwarf_info.lock();
        *info = Some(dwarf_info);
        Ok(())
    }

    /// 生成符号表
    pub fn generate_symbol_table(&self) -> Result<Vec<u8>, SymbolError> {
        let global_symbols = self.global_symbols.lock();
        let mut symbol_table = Vec::new();

        for symbol in global_symbols.values() {
            // 简化实现，实际应该使用适当的二进制格式
            symbol_table.extend_from_slice(&(symbol.name.len() as u32).to_le_bytes());
            symbol_table.extend_from_slice(symbol.name.as_bytes());
            symbol_table.extend_from_slice(&(symbol.address as u64).to_le_bytes());
            symbol_table.extend_from_slice(&(symbol.size as u64).to_le_bytes());
            symbol_table.extend_from_slice(&(symbol.symbol_type as u8).to_le_bytes());
        }

        Ok(symbol_table)
    }

    /// 导出符号表
    pub fn export_symbols(&self, format: ExportFormat) -> Result<String, SymbolError> {
        let global_symbols = self.global_symbols.lock();
        let mut output = String::new();

        match format {
            ExportFormat::Text => {
                for symbol in global_symbols.values() {
                    output.push_str(&format!(
                        "{:#018x} {:#018x} {} {} {}\n",
                        symbol.address,
                        symbol.size,
                        symbol.symbol_type_as_string(),
                        symbol.binding_as_string(),
                        symbol.name
                    ));
                }
            }
            ExportFormat::Json => {
                output.push_str("{\n  \"symbols\": [\n");
                let mut first = true;
                for symbol in global_symbols.values() {
                    if !first {
                        output.push_str(",\n");
                    }
                    first = false;
                    output.push_str(&format!(
                        "    {{\"name\": \"{}\", \"address\": {}, \"size\": {}, \"type\": \"{}\"}}",
                        symbol.name,
                        symbol.address,
                        symbol.size,
                        symbol.symbol_type_as_string()
                    ));
                }
                output.push_str("\n  ]\n}");
            }
            ExportFormat::Nm => {
                // nm格式输出
                for symbol in global_symbols.values() {
                    let nm_type = symbol.nm_symbol_type();
                    output.push_str(&format!(
                        "{:018x} {} {}\n",
                        symbol.address,
                        nm_type,
                        symbol.name
                    ));
                }
            }
        }

        Ok(output)
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> SymbolStatistics {
        SymbolStatistics {
            total_symbols: AtomicUsize::new(
                self.statistics.total_symbols.load(Ordering::SeqCst)
            ),
            function_symbols: AtomicUsize::new(
                self.statistics.function_symbols.load(Ordering::SeqCst)
            ),
            variable_symbols: AtomicUsize::new(
                self.statistics.variable_symbols.load(Ordering::SeqCst)
            ),
            type_symbols: AtomicUsize::new(
                self.statistics.type_symbols.load(Ordering::SeqCst)
            ),
            cache_hits: AtomicU64::new(
                self.statistics.cache_hits.load(Ordering::SeqCst)
            ),
            cache_misses: AtomicU64::new(
                self.statistics.cache_misses.load(Ordering::SeqCst)
            ),
            lookups: AtomicU64::new(
                self.statistics.lookups.load(Ordering::SeqCst)
            ),
            total_lookup_time: AtomicU64::new(
                self.statistics.total_lookup_time.load(Ordering::SeqCst)
            ),
        }
    }

    /// 清空缓存
    pub fn clear_cache(&self) {
        let mut cache = self.symbol_cache.lock();
        cache.clear();
    }

    /// 私有辅助方法
    fn find_similar_symbols(&self, name: &str, symbols: &BTreeMap<String, DebugSymbol>) -> Vec<DebugSymbol> {
        let mut similar = Vec::new();

        for (symbol_name, symbol) in symbols {
            if self.calculate_similarity(name, symbol_name) > 0.6 {
                similar.push(symbol.clone());
            }
        }

        similar.sort_by(|a, b| {
            let sim_a = self.calculate_similarity(name, &a.name);
            let sim_b = self.calculate_similarity(name, &b.name);
            sim_b.partial_cmp(&sim_a).unwrap()
        });

        similar.truncate(5); // 只返回前5个最相似的
        similar
    }

    fn calculate_similarity(&self, name1: &str, name2: &str) -> f64 {
        if name1 == name2 {
            return 1.0;
        }

        // 简单的相似度计算：编辑距离
        let len1 = name1.len();
        let len2 = name2.len();
        let max_len = len1.max(len2);

        if max_len == 0 {
            return 1.0;
        }

        let distance = self.edit_distance(name1, name2);
        1.0 - (distance as f64 / max_len as f64)
    }

    fn edit_distance(&self, s1: &str, s2: &str) -> usize {
        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();
        let len1 = chars1.len();
        let len2 = chars2.len();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[len1][len2]
    }

    fn find_source_location(&self, address: usize) -> Option<SourceLocation> {
        let mappings = self.line_mappings.lock();

        // 找到最接近的行号映射
        let mapping = mappings.iter()
            .filter(|m| m.address <= address)
            .max_by_key(|m| m.address);

        mapping.map(|m| SourceLocation {
            file_path: m.file_path.clone(),
            line: m.line_number,
            column: m.column_number,
            function_name: None, // 需要从符号表查找
            line_content: None,   // 需要读取文件内容
        })
    }

    fn find_containing_function(&self, address: usize) -> Option<FunctionRange> {
        let ranges = self.function_ranges.lock();

        ranges.iter()
            .find(|(_, range)| address >= range.start_address && address <= range.end_address)
            .map(|(_, range)| range.clone())
    }
}

impl DebugSymbol {
    /// 获取符号类型的字符串表示
    pub fn symbol_type_as_string(&self) -> &'static str {
        match self.symbol_type {
            SymbolType::Function => "FUNC",
            SymbolType::Variable => "VAR",
            SymbolType::Type => "TYPE",
            SymbolType::Constant => "CONST",
            SymbolType::Label => "LABEL",
            SymbolType::Unknown => "UNKNOWN",
        }
    }

    /// 获取绑定类型的字符串表示
    pub fn binding_as_string(&self) -> &'static str {
        match self.binding {
            SymbolBinding::Local => "LOCAL",
            SymbolBinding::Global => "GLOBAL",
            SymbolBinding::Weak => "WEAK",
            SymbolBinding::Unknown => "UNKNOWN",
        }
    }

    /// 获取nm格式的符号类型
    pub fn nm_symbol_type(&self) -> char {
        match (self.symbol_type, self.binding) {
            (SymbolType::Function, SymbolBinding::Global) => 'T',
            (SymbolType::Function, SymbolBinding::Local) => 't',
            (SymbolType::Function, SymbolBinding::Weak) => 'W',
            (SymbolType::Variable, SymbolBinding::Global) => 'D',
            (SymbolType::Variable, SymbolBinding::Local) => 'd',
            (SymbolType::Variable, SymbolBinding::Weak) => 'V',
            _ => '?',
        }
    }
}

/// 导出格式
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    /// 文本格式
    Text,
    /// JSON格式
    Json,
    /// nm格式
    Nm,
}

/// 符号错误类型
#[derive(Debug, Clone)]
pub enum SymbolError {
    /// 符号已存在
    SymbolAlreadyExists(String),
    /// 符号不存在
    SymbolNotFound(String),
    /// 无效的符号格式
    InvalidSymbolFormat(String),
    /// DWARF解析错误
    DwarfParseError(String),
    /// 内存不足
    OutOfMemory,
    /// IO错误
    IoError(String),
}

impl core::fmt::Display for SymbolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SymbolError::SymbolAlreadyExists(name) => write!(f, "符号已存在: {}", name),
            SymbolError::SymbolNotFound(name) => write!(f, "符号不存在: {}", name),
            SymbolError::InvalidSymbolFormat(msg) => write!(f, "无效的符号格式: {}", msg),
            SymbolError::DwarfParseError(msg) => write!(f, "DWARF解析错误: {}", msg),
            SymbolError::OutOfMemory => write!(f, "内存不足"),
            SymbolError::IoError(msg) => write!(f, "IO错误: {}", msg),
        }
    }
}

/// 全局符号表管理器实例
static SYMBOL_MANAGER: spin::Mutex<Option<Arc<SymbolTableManager>>> = spin::Mutex::new(None);

/// 初始化调试符号子系统
pub fn init() -> Result<(), SymbolError> {
    let manager = SymbolTableManager::new();

    // 添加内核内置符号
    add_builtin_symbols(&manager)?;

    let manager = Arc::new(manager);
    let mut global_manager = SYMBOL_MANAGER.lock();
    *global_manager = Some(manager);

    crate::println!("[symbols] 调试符号子系统初始化完成");
    Ok(())
}

/// 添加内置符号
fn add_builtin_symbols(manager: &SymbolTableManager) -> Result<(), SymbolError> {
    // 添加内核主函数符号
    manager.add_symbol(DebugSymbol {
        name: "rust_main".to_string(),
        address: rust_main as usize,
        size: 4096, // 简化的大小
        symbol_type: SymbolType::Function,
        binding: SymbolBinding::Global,
        visibility: SymbolVisibility::Default,
        module: Some("kernel".to_string()),
        source_file: Some("main.rs".to_string()),
        line_number: Some(82),
        column_number: Some(1),
        additional_info: BTreeMap::new(),
    })?;

    // 添加一些示例符号
    manager.add_symbol(DebugSymbol {
        name: "panic_handler".to_string(),
        address: 0x10000, // 示例地址
        size: 512,
        symbol_type: SymbolType::Function,
        binding: SymbolBinding::Global,
        visibility: SymbolVisibility::Default,
        module: Some("kernel".to_string()),
        source_file: Some("main.rs".to_string()),
        line_number: Some(387),
        column_number: Some(1),
        additional_info: BTreeMap::new(),
    })?;

    Ok(())
}

/// 获取全局符号表管理器
pub fn get_symbol_manager() -> Result<Arc<SymbolTableManager>, SymbolError> {
    let manager = SYMBOL_MANAGER.lock();
    manager.as_ref()
        .cloned()
        .ok_or(SymbolError::SymbolNotFound("符号管理器未初始化".to_string()))
}

/// 导出函数
unsafe extern "C" {
    fn rust_main() -> !;
}