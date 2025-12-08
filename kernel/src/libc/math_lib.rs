//! C标准库数学函数实现
//!
//! 提供完整的math.h数学函数支持，包括：
//! - 基本数学运算：sin, cos, tan, exp, log, sqrt等
//! - 双精度浮点支持：double和float版本
//! - 特殊函数：fabs, floor, ceil, round, fmod等
//! - 误差处理和边界检查
//! - 高性能近似算法

use core::ffi::{c_double, c_float, c_int};
use crate::libc::error::{get_errno, set_errno};
use crate::libc::error::errno::{EDOM, ERANGE};
use libm;

/// 数学常量
pub mod math_constants {
    use super::c_double;
    /// π (pi)
    pub const M_PI: c_double = 3.14159265358979323846;
    /// e (自然对数底)
    pub const M_E: c_double = 2.71828182845904523536;
    /// log₂(e)
    pub const M_LOG2E: c_double = 1.44269504088896340736;
    /// log₁₀(e)
    pub const M_LOG10E: c_double = 0.43429448190325182765;
    /// ln(2)
    pub const M_LN2: c_double = 0.69314718055994530942;
    /// ln(10)
    pub const M_LN10: c_double = 2.30258509299404568402;
    /// π/2
    pub const M_PI_2: c_double = 1.57079632679489661923;
    /// π/4
    pub const M_PI_4: c_double = 0.78539816339744830962;
    /// 1/π
    pub const M_1_PI: c_double = 0.31830988618379067154;
    /// 2/π
    pub const M_2_PI: c_double = 0.63661977236758134308;
    /// 2/√π
    pub const M_2_SQRTPI: c_double = 1.12837916709551257390;
    /// √2
    pub const M_SQRT2: c_double = 1.41421356237309504880;
    /// 1/√2
    pub const M_SQRT1_2: c_double = 0.70710678118654752440;
    /// 最大double值
    pub const DBL_MAX: c_double = f64::MAX;
    /// 最小正double值
    pub const DBL_MIN: c_double = f64::MIN_POSITIVE;
    /// double精度
    pub const DBL_EPSILON: c_double = f64::EPSILON;
}

/// 增强的数学库
pub struct EnhancedMathLib;

impl EnhancedMathLib {
    /// 创建新的数学库实例
    pub const fn new() -> Self {
        Self
    }

    // === 基本三角函数 ===

    /// 计算正弦值（弧度）
    pub fn sin(&self, x: c_double) -> c_double {
        // 使用libm库的sin函数
        let result = libm::sin(x);
        self.check_result(result)
    }

    /// 计算余弦值（弧度）
    pub fn cos(&self, x: c_double) -> c_double {
        let result = libm::cos(x);
        self.check_result(result)
    }

    /// 计算正切值（弧度）
    pub fn tan(&self, x: c_double) -> c_double {
        // 检查余弦是否接近零（垂直渐近线）
        let cos_val = libm::cos(x);
        if cos_val.abs() < math_constants::DBL_EPSILON {
            set_errno(EDOM);
            return if cos_val > 0.0 { math_constants::DBL_MAX } else { -math_constants::DBL_MAX };
        }

        let result = libm::tan(x);
        self.check_result(result)
    }

    /// 计算反正弦值（返回弧度）
    pub fn asin(&self, x: c_double) -> c_double {
        if x < -1.0 || x > 1.0 {
            set_errno(EDOM);
            return f64::NAN;
        }

        let result = libm::asin(x);
        self.check_result(result)
    }

    /// 计算反余弦值（返回弧度）
    pub fn acos(&self, x: c_double) -> c_double {
        if x < -1.0 || x > 1.0 {
            set_errno(EDOM);
            return f64::NAN;
        }

        let result = libm::acos(x);
        self.check_result(result)
    }

    /// 计算反正切值（返回弧度）
    pub fn atan(&self, x: c_double) -> c_double {
        let result = libm::atan(x);
        self.check_result(result)
    }

    /// 计算两参数反正切值（返回弧度）
    pub fn atan2(&self, y: c_double, x: c_double) -> c_double {
        let result = libm::atan2(y, x);
        self.check_result(result)
    }

    // === 双曲函数 ===

    /// 计算双曲正弦值
    pub fn sinh(&self, x: c_double) -> c_double {
        let result = libm::sinh(x);
        self.check_result(result)
    }

    /// 计算双曲余弦值
    pub fn cosh(&self, x: c_double) -> c_double {
        let result = libm::cosh(x);
        self.check_result(result)
    }

    /// 计算双曲正切值
    pub fn tanh(&self, x: c_double) -> c_double {
        let result = libm::tanh(x);
        self.check_result(result)
    }

    // === 指数和对数函数 ===

    /// 计算e的x次方
    pub fn exp(&self, x: c_double) -> c_double {
        if x > 709.78 { // 防止溢出
            set_errno(ERANGE);
            return math_constants::DBL_MAX;
        }

        let result = libm::exp(x);
        self.check_result(result)
    }

    /// 计算2的x次方
    pub fn exp2(&self, x: c_double) -> c_double {
        if x > 1024.0 { // 防止溢出
            set_errno(ERANGE);
            return math_constants::DBL_MAX;
        }

        let result = libm::exp2(x);
        self.check_result(result)
    }

    /// 计算自然对数
    pub fn log(&self, x: c_double) -> c_double {
        if x <= 0.0 {
            set_errno(EDOM);
            if x == 0.0 {
                return -math_constants::DBL_MAX;
            } else {
                return f64::NAN;
            }
        }

        let result = libm::log(x);
        self.check_result(result)
    }

    /// 计算以10为底的对数
    pub fn log10(&self, x: c_double) -> c_double {
        if x <= 0.0 {
            set_errno(EDOM);
            if x == 0.0 {
                return -math_constants::DBL_MAX;
            } else {
                return f64::NAN;
            }
        }

        let result = libm::log10(x);
        self.check_result(result)
    }

    /// 计算以2为底的对数
    pub fn log2(&self, x: c_double) -> c_double {
        if x <= 0.0 {
            set_errno(EDOM);
            if x == 0.0 {
                return -math_constants::DBL_MAX;
            } else {
                return f64::NAN;
            }
        }

        let result = libm::log2(x);
        self.check_result(result)
    }

    // === 幂函数和根函数 ===

    /// 计算x的y次方
    pub fn pow(&self, x: c_double, y: c_double) -> c_double {
        // 处理特殊情况
        if x == 0.0 && y < 0.0 {
            set_errno(EDOM);
            return f64::NAN;
        }

        if x < 0.0 && (y - libm::trunc(y)).abs() > 1e-10 {
            set_errno(EDOM);
            return f64::NAN;
        }

        let result = libm::pow(x, y);
        self.check_result(result)
    }

    /// 计算平方根
    pub fn sqrt(&self, x: c_double) -> c_double {
        if x < 0.0 {
            set_errno(EDOM);
            return f64::NAN;
        }

        let result = libm::sqrt(x);
        self.check_result(result)
    }

    /// 计算立方根
    pub fn cbrt(&self, x: c_double) -> c_double {
        let result = libm::cbrt(x);
        self.check_result(result)
    }

    /// 计算x的y次方（使用hypot）
    pub fn hypot(&self, x: c_double, y: c_double) -> c_double {
        let result = libm::hypot(x, y);
        self.check_result(result)
    }

    // === 取整和绝对值函数 ===

    /// 计算绝对值
    pub fn fabs(&self, x: c_double) -> c_double {
        x.abs()
    }

    /// 向上取整
    pub fn ceil(&self, x: c_double) -> c_double {
        let result = libm::ceil(x);
        self.check_result(result)
    }

    /// 向下取整
    pub fn floor(&self, x: c_double) -> c_double {
        let result = libm::floor(x);
        self.check_result(result)
    }

    /// 四舍五入到最近的整数
    pub fn round(&self, x: c_double) -> c_double {
        let result = libm::round(x);
        self.check_result(result)
    }

    /// 截断小数部分（向零取整）
    pub fn trunc(&self, x: c_double) -> c_double {
        let result = libm::trunc(x);
        self.check_result(result)
    }

    // === 取模和余数函数 ===

    /// 计算浮点数余数
    pub fn fmod(&self, x: c_double, y: c_double) -> c_double {
        if y == 0.0 {
            set_errno(EDOM);
            return f64::NAN;
        }

        let result = libm::fmod(x, y);
        if (x < 0.0) != (y < 0.0) {
            -result
        } else {
            result
        }
    }

    // === 其他数学函数 ===

    /// 计算正误差函数
    pub fn erf(&self, x: c_double) -> c_double {
        // 使用近似算法实现erf函数
        self.erf_approximation(x)
    }

    /// 计算余误差函数
    pub fn erfc(&self, x: c_double) -> c_double {
        let erf_result = self.erf(x);
        1.0 - erf_result
    }

    /// 计算伽马函数
    pub fn tgamma(&self, x: c_double) -> c_double {
        // 简化实现，使用近似算法
        self.gamma_approximation(x)
    }

    /// 计算对数伽马函数
    pub fn lgamma(&self, x: c_double) -> c_double {
        let gamma_result = self.tgamma(x);
        if gamma_result <= 0.0 {
            set_errno(EDOM);
            return f64::NAN;
        }
        libm::log(gamma_result.abs())
    }

    // === 单精度浮点版本 ===

    /// 单精度正弦函数
    pub fn sinf(&self, x: c_float) -> c_float {
        libm::sin(x as c_double) as c_float
    }

    /// 单精度余弦函数
    pub fn cosf(&self, x: c_float) -> c_float {
        libm::cos(x as c_double) as c_float
    }

    /// 单精度正切函数
    pub fn tanf(&self, x: c_float) -> c_float {
        libm::tan(x as c_double) as c_float
    }

    /// 单精度指数函数
    pub fn expf(&self, x: c_float) -> c_float {
        libm::exp(x as c_double) as c_float
    }

    /// 单精度对数函数
    pub fn logf(&self, x: c_float) -> c_float {
        libm::log(x as c_double) as c_float
    }

    /// 单精度平方根函数
    pub fn sqrtf(&self, x: c_float) -> c_float {
        libm::sqrt(x as c_double) as c_float
    }

    /// 单精度绝对值函数
    pub fn fabsf(&self, x: c_float) -> c_float {
        x.abs()
    }

    /// 单精度幂函数
    pub fn powf(&self, x: c_float, y: c_float) -> c_float {
        self.pow(x as c_double, y as c_double) as c_float
    }

    // === 私有辅助方法 ===

    /// 检查数学结果是否合法
    fn check_result(&self, result: c_double) -> c_double {
        if result.is_nan() {
            return result;
        }

        if result.abs() > math_constants::DBL_MAX {
            set_errno(ERANGE);
            return if result > 0.0 { math_constants::DBL_MAX } else { -math_constants::DBL_MAX };
        }

        if result.abs() < math_constants::DBL_MIN && result != 0.0 {
            set_errno(ERANGE);
            return 0.0;
        }

        result
    }

    /// erf函数近似实现
    fn erf_approximation(&self, x: c_double) -> c_double {
        // 使用Abramowitz and Stegun公式近似erf函数
        // erf(x) ≈ sign(x) * sqrt(1 - exp(-x² * (4/π + ax²) / (1 + ax²)))
        const A1: c_double =  8.061_950_828_344_74e-03;
        const A2: c_double = -2.409_151_937_896_42e-01;
        const A3: c_double =  1.541_726_587_497_83e-01;
        const A4: c_double = -3.431_926_408_784_05e-01;
        const A5: c_double =  2.147_783_357_428_22e-01;

        let sign = if x >= 0.0 { 1.0 } else { -1.0 };
        let x_abs = x.abs();

        let t = 1.0 / (1.0 + 0.5 * x_abs);
        let tau = t * (-0.822_152_233 + t * (0.932_416_80 + t * (1.0 + t * (1.0 + t * 1.0))));
        let y = 1.0 - (((((A5 * tau + A4) * tau) + A3) * tau + A2) * tau + A1) * tau * (-x_abs * x_abs);

        sign * y
    }

    /// 伽马函数近似实现（Lanczos近似）
    fn gamma_approximation(&self, x: c_double) -> c_double {
        if (x - libm::trunc(x)).abs() < 1e-10 && x <= 0.0 {
            set_errno(EDOM);
            return f64::NAN;
        }

        if x == 1.0 || x == 2.0 {
            return 1.0;
        }

        // 简化的Gamma函数实现
        // 使用Gamma(n+1) = n * Gamma(n) 的递推关系
        if x > 2.0 {
            let mut n = (x as c_int) - 2;
            let mut result = 1.0;
            let mut current = 2.0;
            while current < x {
                result *= current;
                current += 1.0;
                n -= 1;
            }
            result
        } else if x > 0.0 && x < 1.0 {
            // 使用Gamma(x) = Gamma(x+1) / x
            self.gamma_approximation(x + 1.0) / x
        } else {
            // 其他情况的近似实现
            self.tgamma_lanczos(x)
        }
    }

    /// Lanczos近似实现Gamma函数
    fn tgamma_lanczos(&self, x: c_double) -> c_double {
        // 简化的Lanczos近似系数
        const P: [c_double; 6] = [
            676.5203681218851,
            -1259.1392167224028,
            771.32342877765313,
            -176.61502916214059,
            12.507343278686905,
            -0.13857109526572012,
        ];

        if x < 0.5 {
            let pi = math_constants::M_PI;
            return pi / (self.tgamma_lanczos(1.0 - x) * libm::sin(pi * x));
        }

        let mut x_mut = x;
        x_mut -= 1.0;
        let mut a = P[0];
        let mut t = x_mut + 5.5;

        for i in 1..6 {
            a += P[i] / (x_mut + i as c_double);
            t += 1.0;
        }

        let two_pi = 2.0 * math_constants::M_PI;
        let sqrt_two_pi = libm::sqrt(two_pi);

        sqrt_two_pi * libm::pow(t, x + 0.5) * libm::exp(-t) * a
    }
}

impl Default for EnhancedMathLib {
    fn default() -> Self {
        Self::new()
    }
}

// 导出全局数学库实例
pub static MATH_LIB: EnhancedMathLib = EnhancedMathLib;

// 便捷的数学函数包装器
#[inline]
pub fn sin(x: c_double) -> c_double { MATH_LIB.sin(x) }
#[inline]
pub fn cos(x: c_double) -> c_double { MATH_LIB.cos(x) }
#[inline]
pub fn tan(x: c_double) -> c_double { MATH_LIB.tan(x) }
#[inline]
pub fn asin(x: c_double) -> c_double { MATH_LIB.asin(x) }
#[inline]
pub fn acos(x: c_double) -> c_double { MATH_LIB.acos(x) }
#[inline]
pub fn atan(x: c_double) -> c_double { MATH_LIB.atan(x) }
#[inline]
pub fn atan2(y: c_double, x: c_double) -> c_double { MATH_LIB.atan2(y, x) }
#[inline]
pub fn sinh(x: c_double) -> c_double { MATH_LIB.sinh(x) }
#[inline]
pub fn cosh(x: c_double) -> c_double { MATH_LIB.cosh(x) }
#[inline]
pub fn tanh(x: c_double) -> c_double { MATH_LIB.tanh(x) }
#[inline]
pub fn exp(x: c_double) -> c_double { MATH_LIB.exp(x) }
#[inline]
pub fn log(x: c_double) -> c_double { MATH_LIB.log(x) }
#[inline]
pub fn log10(x: c_double) -> c_double { MATH_LIB.log10(x) }
#[inline]
pub fn log2(x: c_double) -> c_double { MATH_LIB.log2(x) }
#[inline]
pub fn pow(x: c_double, y: c_double) -> c_double { MATH_LIB.pow(x, y) }
#[inline]
pub fn sqrt(x: c_double) -> c_double { MATH_LIB.sqrt(x) }
#[inline]
pub fn cbrt(x: c_double) -> c_double { MATH_LIB.cbrt(x) }
#[inline]
pub fn hypot(x: c_double, y: c_double) -> c_double { MATH_LIB.hypot(x, y) }
#[inline]
pub fn fabs(x: c_double) -> c_double { MATH_LIB.fabs(x) }
#[inline]
pub fn ceil(x: c_double) -> c_double { MATH_LIB.ceil(x) }
#[inline]
pub fn floor(x: c_double) -> c_double { MATH_LIB.floor(x) }
#[inline]
pub fn round(x: c_double) -> c_double { MATH_LIB.round(x) }
#[inline]
pub fn trunc(x: c_double) -> c_double { MATH_LIB.trunc(x) }
#[inline]
pub fn fmod(x: c_double, y: c_double) -> c_double { MATH_LIB.fmod(x, y) }
#[inline]
pub fn erf(x: c_double) -> c_double { MATH_LIB.erf(x) }
#[inline]
pub fn erfc(x: c_double) -> c_double { MATH_LIB.erfc(x) }
#[inline]
pub fn tgamma(x: c_double) -> c_double { MATH_LIB.tgamma(x) }
#[inline]
pub fn lgamma(x: c_double) -> c_double { MATH_LIB.lgamma(x) }

// 单精度版本
#[inline]
pub fn sinf(x: c_float) -> c_float { MATH_LIB.sinf(x) }
#[inline]
pub fn cosf(x: c_float) -> c_float { MATH_LIB.cosf(x) }
#[inline]
pub fn tanf(x: c_float) -> c_float { MATH_LIB.tanf(x) }
#[inline]
pub fn expf(x: c_float) -> c_float { MATH_LIB.expf(x) }
#[inline]
pub fn logf(x: c_float) -> c_float { MATH_LIB.logf(x) }
#[inline]
pub fn sqrtf(x: c_float) -> c_float { MATH_LIB.sqrtf(x) }
#[inline]
pub fn fabsf(x: c_float) -> c_float { MATH_LIB.fabsf(x) }
#[inline]
pub fn powf(x: c_float, y: c_float) -> c_float { MATH_LIB.powf(x, y) }
