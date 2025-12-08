//! C标准库全面测试套件
//!
//! 提供完整的C标准库功能测试，包括：
//! - 内存管理测试
//! - 字符串操作测试
//! - 数学函数测试
//! - 时间函数测试
//! - 随机数测试
//! - 环境变量测试
//! - 系统信息测试
//! - I/O操作测试
//! - 集成测试

use crate::libc::*;
use crate::libc::implementations::{create_unified_c_lib, UnifiedCLib};
use core::ffi::{c_char, c_int, c_uint};

pub type size_t = usize;

/// 测试结果统计
#[derive(Debug, Default)]
pub struct TestResults {
    /// 总测试数
    pub total_tests: u32,
    /// 通过的测试数
    pub passed_tests: u32,
    /// 失败的测试数
    pub failed_tests: u32,
    /// 跳过的测试数
    pub skipped_tests: u32,
    /// 测试错误信息
    pub errors: heapless::Vec<heapless::String<256>, 64>,
}

impl TestResults {
    /// 记录测试结果
    pub fn record_result(&mut self, passed: bool, test_name: &str, error_msg: Option<&str>) {
        self.total_tests += 1;
        if passed {
            self.passed_tests += 1;
            crate::println!("  ✅ {}", test_name);
        } else {
            self.failed_tests += 1;
            crate::println!("  ❌ {}", test_name);
            if let Some(msg) = error_msg {
                crate::println!("     错误: {}", msg);
                self.errors.push(heapless::String::from_str(format!("{}: {}", test_name, msg)).unwrap_or_default()).ok();
            }
        }
    }

    /// 记录跳过的测试
    pub fn record_skip(&mut self, test_name: &str, reason: &str) {
        self.total_tests += 1;
        self.skipped_tests += 1;
        crate::println!("  ⏭️ {} (跳过: {})", test_name, reason);
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f32 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f32 / self.total_tests as f32) * 100.0
        }
    }

    /// 打印测试报告
    pub fn print_report(&self) {
        crate::println!("\n📊 测试结果统计:");
        crate::println!("  总测试数: {}", self.total_tests);
        crate::println!("  通过: {} ({:.1}%)", self.passed_tests, self.success_rate());
        crate::println!("  失败: {}", self.failed_tests);
        crate::println!("  跳过: {}", self.skipped_tests);

        if !self.errors.is_empty() {
            crate::println!("\n❌ 失败的测试:");
            for error in self.errors.iter() {
                crate::println!("  {}", error);
            }
        }
    }
}

/// C标准库全面测试套件
pub struct StandardLibTests {
    /// 测试结果
    results: TestResults,
    /// C库实例
    libc: UnifiedCLib,
}

impl StandardLibTests {
    /// 创建新的测试套件
    pub fn new() -> Self {
        Self {
            results: TestResults::default(),
            libc: create_unified_c_lib(),
        }
    }

    /// 运行所有测试
    pub fn run_all_tests(&mut self) {
        crate::println!("\n🧪 开始C标准库全面测试");
        crate::println!("=====================");

        // 初始化C库
        if let Err(e) = self.libc.initialize() {
            crate::println!("❌ C库初始化失败: {:?}", e);
            return;
        }

        // 运行各模块测试
        self.test_memory_management();
        self.test_string_operations();
        self.test_math_functions();
        self.test_time_functions();
        self.test_random_functions();
        self.test_environment_variables();
        self.test_system_information();
        self.test_io_operations();
        self.test_error_handling();
        self.test_integration();

        // 打印最终报告
        self.results.print_report();
        crate::println!("\n🏁 C标准库测试完成");
    }

    /// 内存管理测试
    fn test_memory_management(&mut self) {
        crate::println!("\n💾 内存管理测试:");

        // 测试malloc
        let ptr = self.libc.malloc(1024);
        let passed = !ptr.is_null();
        self.results.record_result(passed, "malloc分配内存",
            if passed { None } else { Some("malloc返回空指针") });

        // 测试calloc
        let ptr2 = self.libc.calloc(10, 100);
        let passed = !ptr2.is_null();
        self.results.record_result(passed, "calloc清零分配",
            if passed { None } else { Some("calloc返回空指针") });

        // 测试memset
        let result = self.libc.memset(ptr, 0x42, 10);
        let passed = result == ptr;
        self.results.record_result(passed, "memset内存设置",
            if passed { None } else { Some("memset返回指针错误") });

        // 测试realloc
        let ptr3 = self.libc.realloc(ptr, 2048);
        let passed = !ptr3.is_null();
        self.results.record_result(passed, "realloc重新分配",
            if passed { None } else { Some("realloc返回空指针") });

        // 注意：简化实现中不测试free
        self.libc.free(ptr2);
    }

    /// 字符串操作测试
    fn test_string_operations(&mut self) {
        crate::println!("\n📝 字符串操作测试:");

        let mut buffer = [0u8; 256];
        let src = b"Hello, NOS!";

        // 测试strcpy
        let result = self.libc.strcpy(buffer.as_mut_ptr() as *mut c_char, src.as_ptr() as *const c_char);
        let passed = result == buffer.as_mut_ptr() as *mut c_char;
        self.results.record_result(passed, "strcpy字符串复制",
            if passed { None } else { Some("strcpy返回指针错误") });

        // 测试strlen
        let len = self.libc.strlen(src.as_ptr() as *const c_char);
        let passed = len == src.len();
        self.results.record_result(passed, "strlen字符串长度",
            if passed { None } else { Some(format!("长度不匹配: 期望 {}, 实际 {}", src.len(), len)).as_str() });

        // 测试strcmp
        let cmp = self.libc.strcmp(src.as_ptr() as *const c_char, src.as_ptr() as *const c_char);
        let passed = cmp == 0;
        self.results.record_result(passed, "strcmp字符串比较",
            if passed { None } else { Some("strcmp相同字符串比较结果不为0") });

        // 测试strncmp
        let cmp2 = self.libc.strncmp(b"Hello".as_ptr() as *const c_char, b"Help".as_ptr() as *const c_char, 3);
        let passed = cmp2 == 0;
        self.results.record_result(passed, "strncmp前缀比较",
            if passed { None } else { Some("strncmp前缀比较失败") });

        // 测试strcat
        let mut dest_buffer = [0u8; 256];
        dest_buffer[..b"Hello".len()].copy_from_slice(b"Hello");
        self.libc.strcat(dest_buffer.as_mut_ptr() as *mut c_char, b", World!".as_ptr() as *const c_char);
        let result_str = unsafe {
            core::ffi::CStr::from_ptr(dest_buffer.as_ptr() as *const c_char).to_str().unwrap_or("")
        };
        let passed = result_str == "Hello, World!";
        self.results.record_result(passed, "strcat字符串连接",
            if passed { None } else { Some(format!("连接结果错误: {}", result_str)).as_str() });
    }

    /// 数学函数测试
    fn test_math_functions(&mut self) {
        crate::println!("\n🔢 数学函数测试:");

        let math_lib = &crate::libc::math_lib::ENHANCED_MATH_LIB;

        // 测试基本数学函数
        let sin_val = math_lib.sin(0.0);
        let passed = (sin_val - 0.0).abs() < 0.0001;
        self.results.record_result(passed, "sin(0)",
            if passed { None } else { Some(format!("sin(0)应该为0，实际为{}", sin_val)).as_str() });

        let cos_val = math_lib.cos(0.0);
        let passed = (cos_val - 1.0).abs() < 0.0001;
        self.results.record_result(passed, "cos(0)",
            if passed { None } else { Some(format!("cos(0)应该为1，实际为{}", cos_val)).as_str() });

        let exp_val = math_lib.exp(0.0);
        let passed = (exp_val - 1.0).abs() < 0.0001;
        self.results.record_result(passed, "exp(0)",
            if passed { None } else { Some(format!("exp(0)应该为1，实际为{}", exp_val)).as_str() });

        let log_val = math_lib.log(1.0);
        let passed = (log_val - 0.0).abs() < 0.0001;
        self.results.record_result(passed, "log(1)",
            if passed { None } else { Some(format!("log(1)应该为0，实际为{}", log_val)).as_str() });

        let sqrt_val = math_lib.sqrt(4.0);
        let passed = (sqrt_val - 2.0).abs() < 0.0001;
        self.results.record_result(passed, "sqrt(4)",
            if passed { None } else { Some(format!("sqrt(4)应该为2，实际为{}", sqrt_val)).as_str() });

        let pow_val = math_lib.pow(2.0, 3.0);
        let passed = (pow_val - 8.0).abs() < 0.0001;
        self.results.record_result(passed, "pow(2,3)",
            if passed { None } else { Some(format!("pow(2,3)应该为8，实际为{}", pow_val)).as_str() });
    }

    /// 时间函数测试
    fn test_time_functions(&mut self) {
        crate::println!("\n⏰ 时间函数测试:");

        let time_lib = unsafe { &crate::libc::time_lib::TIME_LIB };

        // 测试time函数
        let mut timestamp = 0i64;
        let result = time_lib.time(&mut timestamp);
        let passed = result > 0;
        self.results.record_result(passed, "time获取时间戳",
            if passed { None } else { Some("time函数返回无效时间戳") });

        // 测试gettimeofday
        let mut timeval = crate::libc::time_lib::Timeval { tv_sec: 0, tv_usec: 0 };
        let result = time_lib.gettimeofday(&mut timeval, core::ptr::null_mut());
        let passed = result == 0 && timeval.tv_sec > 0;
        self.results.record_result(passed, "gettimeofday高精度时间",
            if passed { None } else { Some("gettimeofday调用失败") });

        // 测试mktime和localtime
        let mut tm = crate::libc::time_lib::Tm {
            tm_year: 124, // 2024年
            tm_mon: 0,    // 1月
            tm_mday: 1,
            tm_hour: 0,
            tm_min: 0,
            tm_sec: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
        };
        let timestamp2 = time_lib.mktime(&mut tm);
        let passed = timestamp2 > 0;
        self.results.record_result(passed, "mktime时间转换",
            if passed { None } else { Some("mktime转换失败") });

        // 测试strftime
        let mut format_buffer = [0u8; 100];
        let format_result = time_lib.strftime(
            format_buffer.as_mut_ptr() as *mut c_char,
            format_buffer.len(),
            b"%Y-%m-%d %H:%M:%S".as_ptr() as *const c_char,
            &tm
        );
        let passed = format_result > 0;
        self.results.record_result(passed, "strftime时间格式化",
            if passed { None } else { Some("strftime格式化失败") });
    }

    /// 随机数测试
    fn test_random_functions(&mut self) {
        crate::println!("\n🎲 随机数测试:");

        let random_gen = unsafe { &crate::libc::random_lib::RANDOM_GENERATOR };

        // 测试随机数生成
        random_gen.srand(42);
        let val1 = random_gen.rand();
        let val2 = random_gen.rand();
        let passed = val1 >= 0 && val2 >= 0;
        self.results.record_result(passed, "rand随机数生成",
            if passed { None } else { Some("rand生成负数") });

        // 测试随机数一致性
        random_gen.srand(42);
        let val3 = random_gen.rand();
        let val4 = random_gen.rand();
        let passed = val1 == val3 && val2 == val4;
        self.results.record_result(passed, "srand种子一致性",
            if passed { None } else { Some("相同种子产生不同随机数序列") });

        // 测试随机浮点数
        let float_val = random_gen.rand_float();
        let passed = float_val >= 0.0 && float_val < 1.0;
        self.results.record_result(passed, "rand_float浮点随机数",
            if passed { None } else { Some("rand_float超出[0,1)范围") });

        // 测试随机范围
        let range_val = random_gen.rand_between(10, 20);
        let passed = range_val >= 10 && range_val <= 20;
        self.results.record_result(passed, "rand_between范围随机数",
            if passed { None } else { Some("rand_between超出指定范围") });

        // 测试随机字节生成
        let mut buffer = [0u8; 100];
        random_gen.rand_bytes(&mut buffer, buffer.len());
        let all_zero = buffer.iter().all(|&b| b == 0);
        let all_same = buffer.windows(2).all(|w| w[0] == w[1]);
        let passed = !all_zero && !all_same;
        self.results.record_result(passed, "rand_bytes随机字节",
            if passed { None } else { Some("rand_bytes生成的字节不够随机") });
    }

    /// 环境变量测试
    fn test_environment_variables(&mut self) {
        crate::println!("\n🌍 环境变量测试:");

        let env_manager = unsafe { &crate::libc::env_lib::ENV_MANAGER };

        // 测试设置和获取环境变量
        let result = env_manager.setenv(
            b"TEST_VAR\0".as_ptr(),
            b"test_value\0".as_ptr(),
            1
        );
        let passed = result == 0;
        self.results.record_result(passed, "setenv设置环境变量",
            if passed { None } else { Some("setenv设置失败") });

        // 测试获取环境变量
        let value = env_manager.getenv(b"TEST_VAR\0".as_ptr());
        let passed = !value.is_null();
        self.results.record_result(passed, "getenv获取环境变量",
            if passed { None } else { Some("getenv返回空指针") });

        if !value.is_null() {
            let value_str = unsafe {
                core::ffi::CStr::from_ptr(value).to_str().unwrap_or("")
            };
            let passed = value_str == "test_value";
            self.results.record_result(passed, "getenv值匹配",
                if passed { None } else { Some(format!("值不匹配: 期望 'test_value', 实际 '{}'", value_str)).as_str() });
        }

        // 测试删除环境变量
        let result = env_manager.unsetenv(b"TEST_VAR\0".as_ptr());
        let passed = result == 0;
        self.results.record_result(passed, "unsetenv删除环境变量",
            if passed { None } else { Some("unsetenv删除失败") });

        // 测试获取已删除的环境变量
        let deleted_value = env_manager.getenv(b"TEST_VAR\0".as_ptr());
        let passed = deleted_value.is_null();
        self.results.record_result(passed, "getenv已删除变量",
            if passed { None } else { Some("已删除的环境变量仍可获取") });
    }

    /// 系统信息测试
    fn test_system_information(&mut self) {
        crate::println!("\n💻 系统信息测试:");

        let sysinfo = unsafe { &crate::libc::sysinfo_lib::SYSTEM_INFO };

        // 测试uname
        let mut utsname = crate::libc::sysinfo_lib::UtsName::default();
        let result = sysinfo.uname(&mut utsname);
        let passed = result == 0;
        self.results.record_result(passed, "uname系统信息",
            if passed { None } else { Some("uname调用失败") });

        // 测试sysinfo
        let mut info = crate::libc::sysinfo_lib::SysInfo {
            uptime: 0,
            loads: [0; 3],
            totalram: 0,
            freeram: 0,
            sharedram: 0,
            bufferram: 0,
            totalswap: 0,
            freeswap: 0,
            procs: 0,
            totalhigh: 0,
            freehigh: 0,
            mem_unit: 0,
        };
        let result = sysinfo.sysinfo(&mut info);
        let passed = result == 0 && info.uptime > 0;
        self.results.record_result(passed, "sysinfo系统统计",
            if passed { None } else { Some("sysinfo调用失败或返回无效数据") });

        // 测试gethostname
        let mut hostname_buffer = [0u8; 256];
        let result = sysinfo.gethostname(hostname_buffer.as_mut_ptr() as *mut c_char, hostname_buffer.len());
        let passed = result == 0;
        self.results.record_result(passed, "gethostname主机名",
            if passed { None } else { Some("gethostname调用失败") });

        // 测试getloadavg
        let mut loadavg = [0.0; 3];
        let result = sysinfo.getloadavg(&mut loadavg[0], 3);
        let passed = result > 0;
        self.results.record_result(passed, "getloadavg负载平均",
            if passed { None } else { Some("getloadavg调用失败") });

        // 测试CPU信息
        let cpu_info = sysinfo.get_cpu_info();
        let passed = !cpu_info.architecture.as_str().is_empty() && cpu_info.cores > 0;
        self.results.record_result(passed, "get_cpu_info CPU信息",
            if passed { None } else { Some("CPU信息无效") });

        // 测试内存信息
        let mem_info = sysinfo.get_memory_info();
        let passed = mem_info.total_memory > 0 && mem_info.available_memory > 0;
        self.results.record_result(passed, "get_memory_info 内存信息",
            if passed { None } else { Some("内存信息无效") });
    }

    /// I/O操作测试
    fn test_io_operations(&mut self) {
        crate::println!("\n📁 I/O操作测试:");

        // 测试printf
        let result = self.libc.printf(b"Test message: %s %d\n".as_ptr(), "hello", 42);
        let passed = result > 0;
        self.results.record_result(passed, "printf格式化输出",
            if passed { None } else { Some("printf调用失败") });

        // 测试puts
        let result = self.libc.puts(b"Test puts\n".as_ptr());
        let passed = result > 0;
        self.results.record_result(passed, "puts字符串输出",
            if passed { None } else { Some("puts调用失败") });

        // 测试putchar
        let result = self.libc.putchar('A' as c_int);
        let passed = result == 'A' as c_int;
        self.results.record_result(passed, "putchar字符输出",
            if passed { None } else { Some("putchar返回值错误") });

        // 测试getchar（简化实现返回换行符）
        let result = self.libc.getchar();
        let passed = result == '\n' as c_int;
        self.results.record_result(passed, "getchar字符输入",
            if passed { None } else { Some("getchar返回值不符合预期") });
    }

    /// 错误处理测试
    fn test_error_handling(&mut self) {
        crate::println!("\n⚠️ 错误处理测试:");

        // 测试errno设置和获取
        crate::libc::error::set_errno(crate::libc::error::errno::ENOENT);
        let current_errno = crate::libc::error::get_errno();
        let passed = current_errno == crate::libc::error::errno::ENOENT;
        self.results.record_result(passed, "errno错误码设置",
            if passed { None } else { Some("errno设置或获取失败") });

        // 测试strerror
        let error_msg = crate::libc::error::strerror(crate::libc::error::errno::ENOENT);
        let passed = !error_msg.is_empty();
        self.results.record_result(passed, "strerror错误消息",
            if passed { None } else { Some("strerror返回空消息") });

        // 测试清零errno
        crate::libc::error::clear_errno();
        let cleared_errno = crate::libc::error::get_errno();
        let passed = cleared_errno == 0;
        self.results.record_result(passed, "clear_errno清零错误码",
            if passed { None } else { Some("clear_errno清零失败") });
    }

    /// 集成测试
    fn test_integration(&mut self) {
        crate::println!("\n🔗 集成测试:");

        // 测试复杂的字符串格式化和数学计算
        let math_lib = &crate::libc::math_lib::ENHANCED_MATH_LIB;
        let angle = math_lib.pi() / 4.0; // 45度
        let sin_val = math_lib.sin(angle);
        let cos_val = math_lib.cos(angle);

        let result = self.libc.printf(b"sin(\xCF\x80/4) = %.3f, cos(\xCF\x80/4) = %.3f\n".as_ptr(), sin_val, cos_val);
        let passed = result > 0 && (sin_val - 0.707).abs() < 0.01 && (cos_val - 0.707).abs() < 0.01;
        self.results.record_result(passed, "数学计算和格式化集成",
            if passed { None } else { Some("数学计算和格式化集成测试失败") });

        // 测试内存分配、字符串操作和环境变量集成
        let ptr = self.libc.malloc(256);
        let passed = !ptr.is_null();
        self.results.record_result(passed, "内存分配集成",
            if passed { None } else { Some("集成测试中的内存分配失败") });

        if !ptr.is_null() {
            let test_str = b"Integration test string";
            let result = self.libc.strcpy(ptr as *mut c_char, test_str.as_ptr() as *const c_char);
            let passed = result == ptr as *mut c_char;
            self.results.record_result(passed, "字符串复制集成",
                if passed { None } else { Some("集成测试中的字符串复制失败") });

            let len = self.libc.strlen(ptr as *const c_char);
            let passed = len == test_str.len();
            self.results.record_result(passed, "字符串长度集成",
                if passed { None } else { Some("集成测试中的字符串长度检查失败") });
        }

        // 测试时间获取和格式化集成
        let time_lib = unsafe { &crate::libc::time_lib::TIME_LIB };
        let mut timestamp = 0i64;
        let result = time_lib.time(&mut timestamp);
        let passed = result > 0;
        self.results.record_result(passed, "时间获取集成",
            if passed { None } else { Some("集成测试中的时间获取失败") });

        if result > 0 {
            let tm_ptr = time_lib.localtime(&timestamp);
            let passed = !tm_ptr.is_null();
            self.results.record_result(passed, "时间转换集成",
                if passed { None } else { Some("集成测试中的时间转换失败") });

            if !tm_ptr.is_null() {
                let mut format_buffer = [0u8; 100];
                let format_result = time_lib.strftime(
                    format_buffer.as_mut_ptr() as *mut c_char,
                    format_buffer.len(),
                    b"%Y-%m-%d %H:%M:%S".as_ptr() as *const c_char,
                    tm_ptr
                );
                let passed = format_result > 0;
                self.results.record_result(passed, "时间格式化集成",
                    if passed { None } else { Some("集成测试中的时间格式化失败") });
            }
        }
    }
}

/// 运行标准库测试的便捷函数
pub fn run_standard_lib_tests() {
    let mut test_suite = StandardLibTests::new();
    test_suite.run_all_tests();
}

/// 运行单个模块测试的便捷函数
pub mod module_tests {
    use super::*;

    pub fn run_memory_tests() {
        let mut test_suite = StandardLibTests::new();
        test_suite.test_memory_management();
        test_suite.results.print_report();
    }

    pub fn run_string_tests() {
        let mut test_suite = StandardLibTests::new();
        test_suite.test_string_operations();
        test_suite.results.print_report();
    }

    pub fn run_math_tests() {
        let mut test_suite = StandardLibTests::new();
        test_suite.test_math_functions();
        test_suite.results.print_report();
    }

    pub fn run_time_tests() {
        let mut test_suite = StandardLibTests::new();
        test_suite.test_time_functions();
        test_suite.results.print_report();
    }

    pub fn run_random_tests() {
        let mut test_suite = StandardLibTests::new();
        test_suite.test_random_functions();
        test_suite.results.print_report();
    }

    pub fn run_env_tests() {
        let mut test_suite = StandardLibTests::new();
        test_suite.test_environment_variables();
        test_suite.results.print_report();
    }

    pub fn run_sysinfo_tests() {
        let mut test_suite = StandardLibTests::new();
        test_suite.test_system_information();
        test_suite.results.print_report();
    }

    pub fn run_io_tests() {
        let mut test_suite = StandardLibTests::new();
        test_suite.test_io_operations();
        test_suite.results.print_report();
    }

    pub fn run_error_tests() {
        let mut test_suite = StandardLibTests::new();
        test_suite.test_error_handling();
        test_suite.results.print_report();
    }

    pub fn run_integration_tests() {
        let mut test_suite = StandardLibTests::new();
        test_suite.test_integration();
        test_suite.results.print_report();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_suite() {
        run_standard_lib_tests();
    }

    #[test]
    fn test_individual_modules() {
        module_tests::run_memory_tests();
        module_tests::run_string_tests();
        module_tests::run_math_tests();
        module_tests::run_time_tests();
        module_tests::run_random_tests();
        module_tests::run_env_tests();
        module_tests::run_sysinfo_tests();
        module_tests::run_io_tests();
        module_tests::run_error_tests();
        module_tests::run_integration_tests();
    }
}
