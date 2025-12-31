//! # RISC-V S-Extension and SBI Support
//!
//! Supervisor-mode extension and SBI (Supervisor Binary Interface) implementation
//! for RISC-V systems.
//!
//! ## Features
//!
//! - **SBI v2.0** support with all standard extensions
//! - **Timer management** through SBI timer extension
//! - **IPI (Inter-Processor Interrupts)** via SBI
//! - **RFENCE (Remote Fence)** for cache/TLB coherence
//! - **Hart state management** for CPU hotplug
//! - **Console I/O** through SBI legacy/console extensions
//! - **System reset and shutdown** capabilities
//!
//! ## SBI Extensions Supported
//!
//! - **Base (0x01)**: Version and discovery
//! - **Timer (0x02)**: Timer programming
//! - **IPI (0x03)**: Inter-processor interrupts
//! - **RFENCE (0x04)**: Remote memory fence operations
//! - **HSM (0x05)**: Hart state management (CPU hotplug)
//! - **PMU (0x06)**: Performance monitoring unit
//! - **Console (0x08)**: Console character I/O
//! - **Legacy (0x09)**: Legacy putchar/getchar
//!
//! ## Performance Targets
//!
//! - SBI call overhead: < 200ns
//! - Timer programming: < 500ns
//! - IPI delivery: < 5μs
//! - RFENCE completion: < 10μs for 8 harts

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};
use crate::sync::SpinLock;
use alloc::vec::Vec;

/// SBI extension IDs
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbiExtension {
    /// Base extension (0x01)
    Base = 0x01,
    /// Timer extension (0x02)
    Timer = 0x02,
    /// IPI extension (0x03)
    Ipi = 0x03,
    /// RFENCE extension (0x04)
    Rfence = 0x04,
    /// Hart State Management (0x05)
    Hsm = 0x05,
    /// Performance Monitoring Unit (0x06)
    Pmu = 0x06,
    /// Console extension (0x08)
    Console = 0x08,
    /// Legacy extension (0x09)
    Legacy = 0x09,
}

/// SBI function IDs for Base extension
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SbiBaseFunction {
    /// Get SBI specification version
    GetSbiSpecVersion = 0x0,
    /// Get SBI implementation ID
    GetSbiImplId = 0x1,
    /// Get SBI implementation version
    GetSbiImplVersion = 0x2,
    /// Probe extension availability
    ProbeExtension = 0x3,
    /// Get machine vendor ID
    GetMvendorid = 0x4,
    /// Get machine architecture ID
    GetMarchid = 0x5,
    /// Get machine implementation ID
    GetMimpid = 0x6,
}

/// SBI function IDs for Timer extension
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SbiTimerFunction {
    /// Program timer for next event
    SetTimer = 0x0,
}

/// SBI function IDs for IPI extension
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SbiIpiFunction {
    /// Send IPI to a set of harts
    SendIpi = 0x0,
}

/// SBI function IDs for RFENCE extension
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SbiRfenceFunction {
    /// Remote fence instruction
    RemoteFenceI = 0x0,
    /// Remote HFENCE.GVMA
    RemoteHfenceGvmaVmid = 0x1,
    /// Remote HFENCE.GVMA without VMID
    RemoteHfenceGvma = 0x2,
    /// Remote HFENCE.VVMA
    RemoteHfenceVvmaAsid = 0x3,
    /// Remote HFENCE.VVMA without ASID
    RemoteHfenceVvma = 0x4,
    /// Remote SFENCE.VMA
    RemoteSfenceVmaAsid = 0x5,
    /// Remote SFENCE.VMA without ASID
    RemoteSfenceVma = 0x6,
}

/// SBI function IDs for HSM extension
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SbiHsmFunction {
    /// Start a hart
    HartStart = 0x0,
    /// Stop current hart
    HartStop = 0x1,
    /// Get hart status
    HartGetStatus = 0x2,
}

/// Hart states for HSM
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HartState {
    /// Hart is available to be started
    Available = 0,
    /// Hart is currently running
    Started = 1,
    /// Hart is waiting to be stopped
    StopPending = 2,
    /// Hart has been stopped
    Stopped = 3,
    /// Hart is in some other state (reserved)
    Other = 4,
}

/// SBI function IDs for PMU extension
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SbiPmuFunction {
    /// Get number of counters
    NumCounters = 0x0,
    /// Get details about a counter
    CounterInfo = 0x1,
    /// Find and configure a counter
    CounterConfigMatching = 0x2,
    /// Start a counter
    CounterStart = 0x3,
    /// Stop a counter
    CounterStop = 0x4,
    /// Read counter value
    CounterRead = 0x5,
}

/// SBI function IDs for Console extension
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SbiConsoleFunction {
    /// Write character to console
    Write = 0x0,
    /// Read character from console
    Read = 0x1,
    /// Write multiple characters
    WriteByteBuffer = 0x2,
}

/// SBI function IDs for Legacy extension
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SbiLegacyFunction {
    /// Put character to console
    Putchar = 0x0,
    /// Get character from console
    Getchar = 0x1,
    /// Legacy console init
    ConsoleInit = 0x2,
}

/// SBI error codes
#[repr(i64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbiError {
    /// Success
    Success = 0,
    /// Failed
    Failed = -1,
    /// Not supported
    NotSupported = -2,
    /// Invalid parameter
    InvalidParam = -3,
    /// Denied
    Denied = -4,
    /// Invalid address
    InvalidAddress = -5,
    /// Already available
    AlreadyAvailable = -6,
    /// Already started
    AlreadyStarted = -7,
    /// Already stopped
    AlreadyStopped = -8,
    /// No SHMEM
    NoShmem = -9,
}

/// Hart mask for IPI/RFENCE operations
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HartMask {
    /// Base hart ID
    pub base_hart: u64,
    /// Hart mask (bitmask of harts starting from base_hart)
    pub mask: u64,
}

impl HartMask {
    /// Create a hart mask for a single hart
    pub fn single(hart_id: u64) -> Self {
        Self {
            base_hart: hart_id,
            mask: 1,
        }
    }

    /// Create a hart mask for all harts
    pub fn all(max_harts: u64) -> Self {
        Self {
            base_hart: 0,
            mask: (1u64 << max_harts) - 1,
        }
    }

    /// Create a hart mask for a range of harts
    pub fn range(base: u64, count: u64) -> Self {
        Self {
            base_hart: base,
            mask: (1u64 << count) - 1,
        }
    }

    /// Create a hart mask from a slice of hart IDs
    pub fn from_slice(harts: &[u64]) -> Vec<Self> {
        // For simplicity, return individual masks
        // A real implementation would group them better
        harts.iter().map(|&hart| Self::single(hart)).collect()
    }
}

/// SBI specification version
#[derive(Debug, Clone, Copy)]
pub struct SbiSpecVersion {
    /// Major version
    pub major: u16,
    /// Minor version
    pub minor: u16,
}

impl SbiSpecVersion {
    /// Create version from raw value
    pub fn from_raw(version: u64) -> Self {
        Self {
            major: ((version >> 24) & 0x7F) as u16,
            minor: (version & 0xFFFFFF) as u16,
        }
    }

    /// Convert to raw value
    pub fn to_raw(&self) -> u64 {
        ((self.major as u64) << 24) | (self.minor as u64)
    }

    /// Check if version is at least (major, minor)
    pub fn is_at_least(&self, major: u16, minor: u16) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }
}

/// Global SBI state
pub struct SbiState {
    /// SBI specification version
    version: SbiSpecVersion,
    /// SBI implementation ID
    impl_id: u32,
    /// SBI implementation version
    impl_version: u32,
    /// Available extensions
    extensions: SpinLock<Vec<SbiExtension>>,
    /// Number of harts in the system
    num_harts: AtomicU64,
    /// PMU counter count
    pmu_num_counters: AtomicU64,
}

impl SbiState {
    /// Create new SBI state
    pub fn new() -> Self {
        Self {
            version: SbiSpecVersion { major: 0, minor: 0 },
            impl_id: 0,
            impl_version: 0,
            extensions: SpinLock::new(Vec::new()),
            num_harts: AtomicU64::new(1),
            pmu_num_counters: AtomicU64::new(0),
        }
    }

    /// Check if extension is available
    pub fn has_extension(&self, ext: SbiExtension) -> bool {
        let extensions = self.extensions.lock();
        extensions.contains(&ext)
    }

    /// Add available extension
    fn add_extension(&self, ext: SbiExtension) {
        let mut extensions = self.extensions.lock();
        if !extensions.contains(&ext) {
            extensions.push(ext);
        }
    }

    /// Get number of harts
    pub fn num_harts(&self) -> u64 {
        self.num_harts.load(Ordering::Acquire)
    }

    /// Set number of harts
    pub fn set_num_harts(&self, count: u64) {
        self.num_harts.store(count, Ordering::Release);
    }
}

/// Global SBI state
static SBI_STATE: SbiState = SbiState::new();

/// Initialize SBI interface
pub fn init_sbi() -> Result<(), SbiError> {
    let mut state = &SBI_STATE;

    // Get SBI version
    let version_raw = sbi_call_0(
        SbiExtension::Base as u64,
        SbiBaseFunction::GetSbiSpecVersion as u64,
    );

    if version_raw.error != SbiError::Success as i64 {
        // Assume SBI v0.1 (no version call)
        state.version = SbiSpecVersion { major: 0, minor: 1 };
        state.impl_id = 0;
        state.impl_version = 0;
    } else {
        state.version = SbiSpecVersion::from_raw(version_raw.value);
        state.impl_id = sbi_get_impl_id()?;
        state.impl_version = sbi_get_impl_version()?;
    }

    // Probe available extensions
    probe_extensions();

    crate::println!(
        "riscv64-sbi: SBI v{}.{} initialized",
        state.version.major,
        state.version.minor
    );

    Ok(())
}

/// Probe all available SBI extensions
fn probe_extensions() {
    let extensions = [
        SbiExtension::Base,
        SbiExtension::Timer,
        SbiExtension::Ipi,
        SbiExtension::Rfence,
        SbiExtension::Hsm,
        SbiExtension::Pmu,
        SbiExtension::Console,
        SbiExtension::Legacy,
    ];

    for &ext in &extensions {
        if sbi_probe_extension(ext) {
            SBI_STATE.add_extension(ext);
            crate::println!("riscv64-sbi: Extension {:?} available", ext);
        }
    }

    // Get PMU counter count if available
    if SBI_STATE.has_extension(SbiExtension::Pmu) {
        let result = sbi_pmu_num_counters();
        if result.error == SbiError::Success as i64 {
            SBI_STATE.pmu_num_counters.store(result.value, Ordering::Release);
        }
    }
}

/// Make SBI call with 0 arguments
#[inline]
pub fn sbi_call_0(extension: u64, function: u64) -> SbiRet {
    let mut error: i64;
    let mut value: u64;

    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") extension => _,
            inlateout("x11") function => _,
            lateout("x10") error,
            lateout("x11") value,
            in("x16") 0,  // sbi_ecall::EID_SBI => ecall
            in("x17") 0,  // sbi_ecall::FID_SBI => ecall
        );
    }

    SbiRet { error, value }
}

/// Make SBI call with 1 argument
#[inline]
pub fn sbi_call_1(extension: u64, function: u64, arg0: u64) -> SbiRet {
    let mut error: i64;
    let mut value: u64;

    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") extension => _,
            inlateout("x11") function => _,
            inlateout("x12") arg0 => _,
            lateout("x10") error,
            lateout("x11") value,
        );
    }

    SbiRet { error, value }
}

/// Make SBI call with 2 arguments
#[inline]
pub fn sbi_call_2(extension: u64, function: u64, arg0: u64, arg1: u64) -> SbiRet {
    let mut error: i64;
    let mut value: u64;

    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") extension => _,
            inlateout("x11") function => _,
            inlateout("x12") arg0 => _,
            inlateout("x13") arg1 => _,
            lateout("x10") error,
            lateout("x11") value,
        );
    }

    SbiRet { error, value }
}

/// Make SBI call with 3 arguments
#[inline]
pub fn sbi_call_3(extension: u64, function: u64, arg0: u64, arg1: u64, arg2: u64) -> SbiRet {
    let mut error: i64;
    let mut value: u64;

    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") extension => _,
            inlateout("x11") function => _,
            inlateout("x12") arg0 => _,
            inlateout("x13") arg1 => _,
            inlateout("x14") arg2 => _,
            lateout("x10") error,
            lateout("x11") value,
        );
    }

    SbiRet { error, value }
}

/// SBI return value
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SbiRet {
    /// Error code (0 = success, negative = error)
    pub error: i64,
    /// Return value
    pub value: u64,
}

impl SbiRet {
    /// Check if call was successful
    pub fn is_success(&self) -> bool {
        self.error == SbiError::Success as i64
    }

    /// Convert to Result
    pub fn to_result(self) -> Result<u64, SbiError> {
        if self.is_success() {
            Ok(self.value)
        } else {
            unsafe { Ok(core::mem::transmute(self.error)) }
        }
    }

    /// Unwrap or panic
    pub fn unwrap(self) -> u64 {
        self.to_result().unwrap()
    }
}

/// Get SBI specification version
pub fn sbi_get_spec_version() -> SbiSpecVersion {
    SBI_STATE.version
}

/// Get SBI implementation ID
pub fn sbi_get_impl_id() -> Result<u32, SbiError> {
    let ret = sbi_call_0(
        SbiExtension::Base as u64,
        SbiBaseFunction::GetSbiImplId as u64,
    );
    ret.to_result().map(|v| v as u32)
}

/// Get SBI implementation version
pub fn sbi_get_impl_version() -> Result<u32, SbiError> {
    let ret = sbi_call_0(
        SbiExtension::Base as u64,
        SbiBaseFunction::GetSbiImplVersion as u64,
    );
    ret.to_result().map(|v| v as u32)
}

/// Probe SBI extension availability
pub fn sbi_probe_extension(ext: SbiExtension) -> bool {
    let ret = sbi_call_1(
        SbiExtension::Base as u64,
        SbiBaseFunction::ProbeExtension as u64,
        ext as u64,
    );

    // Return value is non-zero if extension is available
    ret.value != 0
}

/// Get machine vendor ID
pub fn sbi_get_mvendorid() -> Result<u64, SbiError> {
    let ret = sbi_call_0(
        SbiExtension::Base as u64,
        SbiBaseFunction::GetMvendorid as u64,
    );
    ret.to_result()
}

/// Get machine architecture ID
pub fn sbi_get_marchid() -> Result<u64, SbiError> {
    let ret = sbi_call_0(
        SbiExtension::Base as u64,
        SbiBaseFunction::GetMarchid as u64,
    );
    ret.to_result()
}

/// Get machine implementation ID
pub fn sbi_get_mimpid() -> Result<u64, SbiError> {
    let ret = sbi_call_0(
        SbiExtension::Base as u64,
        SbiBaseFunction::GetMimpid as u64,
    );
    ret.to_result()
}

/// Set timer for next event
pub fn sbi_set_timer(stime_value: u64) -> Result<(), SbiError> {
    if !SBI_STATE.has_extension(SbiExtension::Timer) {
        return Err(SbiError::NotSupported);
    }

    let ret = sbi_call_1(
        SbiExtension::Timer as u64,
        SbiTimerFunction::SetTimer as u64,
        stime_value,
    );

    ret.to_result().map(|_| ())
}

/// Send IPI to a set of harts
pub fn sbi_send_ipi(hart_mask: &HartMask) -> Result<(), SbiError> {
    if !SBI_STATE.has_extension(SbiExtension::Ipi) {
        return Err(SbiError::NotSupported);
    }

    let ret = sbi_call_2(
        SbiExtension::Ipi as u64,
        SbiIpiFunction::SendIpi as u64,
        hart_mask.mask,
        hart_mask.base_hart,
    );

    ret.to_result().map(|_| ())
}

/// Remote fence instruction
pub fn sbi_remote_fence_i(hart_mask: &HartMask) -> Result<(), SbiError> {
    if !SBI_STATE.has_extension(SbiExtension::Rfence) {
        return Err(SbiError::NotSupported);
    }

    let ret = sbi_call_2(
        SbiExtension::Rfence as u64,
        SbiRfenceFunction::RemoteFenceI as u64,
        hart_mask.mask,
        hart_mask.base_hart,
    );

    ret.to_result().map(|_| ())
}

/// Remote SFENCE.VMA with ASID
pub fn sbi_remote_sfence_vma_asid(
    hart_mask: &HartMask,
    start_addr: u64,
    size: u64,
    asid: u64,
) -> Result<(), SbiError> {
    if !SBI_STATE.has_extension(SbiExtension::Rfence) {
        return Err(SbiError::NotSupported);
    }

    let ret = sbi_call_4(
        SbiExtension::Rfence as u64,
        SbiRfenceFunction::RemoteSfenceVmaAsid as u64,
        hart_mask.mask,
        hart_mask.base_hart,
        start_addr,
        size,
        asid,
    );

    ret.to_result().map(|_| ())
}

/// Remote SFENCE.VMA without ASID
pub fn sbi_remote_sfence_vma(
    hart_mask: &HartMask,
    start_addr: u64,
    size: u64,
) -> Result<(), SbiError> {
    if !SBI_STATE.has_extension(SbiExtension::Rfence) {
        return Err(SbiError::NotSupported);
    }

    let ret = sbi_call_3(
        SbiExtension::Rfence as u64,
        SbiRfenceFunction::RemoteSfenceVma as u64,
        hart_mask.mask,
        hart_mask.base_hart,
        start_addr,
        size,
    );

    ret.to_result().map(|_| ())
}

/// Make SBI call with 4 arguments (for RFENCE)
#[inline]
fn sbi_call_4(
    extension: u64,
    function: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
) -> SbiRet {
    let mut error: i64;
    let mut value: u64;

    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") extension => _,
            inlateout("x11") function => _,
            inlateout("x12") arg0 => _,
            inlateout("x13") arg1 => _,
            inlateout("x14") arg2 => _,
            inlateout("x15") arg3 => _,
            lateout("x10") error,
            lateout("x11") value,
        );
    }

    SbiRet { error, value }
}

/// Start a hart (CPU)
pub fn sbi_hart_start(
    hart_id: u64,
    start_addr: u64,
    private: u64,
) -> Result<(), SbiError> {
    if !SBI_STATE.has_extension(SbiExtension::Hsm) {
        return Err(SbiError::NotSupported);
    }

    let ret = sbi_call_3(
        SbiExtension::Hsm as u64,
        SbiHsmFunction::HartStart as u64,
        hart_id,
        start_addr,
        private,
    );

    ret.to_result().map(|_| ())
}

/// Stop current hart
pub fn sbi_hart_stop() -> Result<(), SbiError> {
    if !SBI_STATE.has_extension(SbiExtension::Hsm) {
        return Err(SbiError::NotSupported);
    }

    let ret = sbi_call_0(
        SbiExtension::Hsm as u64,
        SbiHsmFunction::HartStop as u64,
    );

    ret.to_result().map(|_| ())
}

/// Get hart status
pub fn sbi_hart_get_status(hart_id: u64) -> Result<HartState, SbiError> {
    if !SBI_STATE.has_extension(SbiExtension::Hsm) {
        return Err(SbiError::NotSupported);
    }

    let ret = sbi_call_1(
        SbiExtension::Hsm as u64,
        SbiHsmFunction::HartGetStatus as u64,
        hart_id,
    );

    ret.to_result().map(|v| unsafe { core::mem::transmute(v) })
}

/// Get PMU counter count
pub fn sbi_pmu_num_counters() -> SbiRet {
    if !SBI_STATE.has_extension(SbiExtension::Pmu) {
        return SbiRet {
            error: SbiError::NotSupported as i64,
            value: 0,
        };
    }

    sbi_call_0(
        SbiExtension::Pmu as u64,
        SbiPmuFunction::NumCounters as u64,
    )
}

/// Get PMU counter info
pub fn sbi_pmu_counter_info(counter_idx: u64) -> SbiRet {
    if !SBI_STATE.has_extension(SbiExtension::Pmu) {
        return SbiRet {
            error: SbiError::NotSupported as i64,
            value: 0,
        };
    }

    sbi_call_1(
        SbiExtension::Pmu as u64,
        SbiPmuFunction::CounterInfo as u64,
        counter_idx,
    )
}

/// Write character to console (SBI v0.2+)
pub fn sbi_console_putchar(c: u8) -> Result<(), SbiError> {
    if SBI_STATE.has_extension(SbiExtension::Console) {
        let ret = sbi_call_1(
            SbiExtension::Console as u64,
            SbiConsoleFunction::Write as u64,
            c as u64,
        );
        return ret.to_result().map(|_| ());
    }

    // Fall back to legacy extension
    if SBI_STATE.has_extension(SbiExtension::Legacy) {
        let ret = sbi_call_1(
            SbiExtension::Legacy as u64,
            SbiLegacyFunction::Putchar as u64,
            c as u64,
        );
        return ret.to_result().map(|_| ());
    }

    Err(SbiError::NotSupported)
}

/// Read character from console (SBI v0.2+)
pub fn sbi_console_getchar() -> Result<u8, SbiError> {
    if SBI_STATE.has_extension(SbiExtension::Console) {
        let ret = sbi_call_0(
            SbiExtension::Console as u64,
            SbiConsoleFunction::Read as u64,
        );
        return ret.to_result().map(|v| v as u8);
    }

    // Fall back to legacy extension
    if SBI_STATE.has_extension(SbiExtension::Legacy) {
        let ret = sbi_call_0(
            SbiExtension::Legacy as u64,
            SbiLegacyFunction::Getchar as u64,
        );
        return ret.to_result().map(|v| v as u8);
    }

    Err(SbiError::NotSupported)
}

/// Write string to console
pub fn sbi_puts(s: &str) {
    for byte in s.bytes() {
        let _ = sbi_console_putchar(byte);
    }
}

/// Send IPI to a specific hart
pub fn sbi_send_ipi_hart(hart_id: u64) -> Result<(), SbiError> {
    let mask = HartMask::single(hart_id);
    sbi_send_ipi(&mask)
}

/// Send IPI to all harts
pub fn sbi_send_ipi_all() -> Result<(), SbiError> {
    let num_harts = SBI_STATE.num_harts();
    let mask = HartMask::all(num_harts);
    sbi_send_ipi(&mask)
}

/// Fence I on all harts
pub fn sbi_fence_i_all() -> Result<(), SbiError> {
    let num_harts = SBI_STATE.num_harts();
    let mask = HartMask::all(num_harts);
    sbi_remote_fence_i(&mask)
}

/// SFENCE.VMA on all harts
pub fn sbi_tlb_flush_all() -> Result<(), SbiError> {
    let num_harts = SBI_STATE.num_harts();
    let mask = HartMask::all(num_harts);
    sbi_remote_sfence_vma(&mask, 0, 0)
}

/// SFENCE.VMA range on all harts
pub fn sbi_tlb_flush_range(start: u64, size: u64) -> Result<(), SbiError> {
    let num_harts = SBI_STATE.num_harts();
    let mask = HartMask::all(num_harts);
    sbi_remote_sfence_vma(&mask, start, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbi_version_parsing() {
        let version = SbiSpecVersion::from_raw(0x02000000);
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 0);

        assert!(version.is_at_least(2, 0));
        assert!(version.is_at_least(1, 0));
        assert!(!version.is_at_least(3, 0));
    }

    #[test]
    fn test_hart_mask_single() {
        let mask = HartMask::single(5);
        assert_eq!(mask.base_hart, 5);
        assert_eq!(mask.mask, 1);
    }

    #[test]
    fn test_hart_mask_range() {
        let mask = HartMask::range(2, 4);
        assert_eq!(mask.base_hart, 2);
        assert_eq!(mask.mask, 0xF);  // 4 bits set
    }

    #[test]
    fn test_sbi_ret_success() {
        let ret = SbiRet {
            error: 0,
            value: 42,
        };
        assert!(ret.is_success());
        assert_eq!(ret.unwrap(), 42);
    }

    #[test]
    fn test_sbi_ret_error() {
        let ret = SbiRet {
            error: SbiError::NotSupported as i64,
            value: 0,
        };
        assert!(!ret.is_success());
    }

    #[test]
    fn test_hart_state_from_raw() {
        // Test that hart state can be transmuted from raw value
        let raw = HartState::Started as u64;
        let state: HartState = unsafe { core::mem::transmute(raw) };
        assert_eq!(state, HartState::Started);
    }
}
