//! Memory ECC - Error Correcting Code initialization and management
//!
//! Provides:
//! - ECC initialization
//! - Single-bit error correction
//! - Multi-bit error detection
//! - Error logging and reporting

/// ECC modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EccMode {
    /// ECC disabled
    Disabled,
    /// ECC enabled for detection only
    DetectionOnly,
    /// ECC enabled for correction
    Correction,
}

/// ECC error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EccErrorType {
    /// No error
    NoError,
    /// Single-bit error (correctable)
    SingleBitError,
    /// Multi-bit error (uncorrectable)
    MultiBitError,
}

/// ECC error information
#[derive(Debug, Clone, Copy)]
pub struct EccError {
    /// Error type
    pub error_type: EccErrorType,
    /// Memory address
    pub address: u64,
    /// Syndrome value
    pub syndrome: u32,
    /// Timestamp
    pub timestamp: u64,
    /// CPU that detected error
    pub cpu_id: u32,
    /// Error count
    pub count: u32,
}

impl EccError {
    /// Create ECC error
    pub fn new(error_type: EccErrorType, address: u64) -> Self {
        EccError {
            error_type,
            address,
            syndrome: 0,
            timestamp: 0,
            cpu_id: 0,
            count: 1,
        }
    }

    /// Check if error is correctable
    pub fn is_correctable(&self) -> bool {
        self.error_type == EccErrorType::SingleBitError
    }

    /// Increment error count
    pub fn increment_count(&mut self) {
        self.count += 1;
    }
}

/// ECC DIMM information
#[derive(Debug, Clone, Copy)]
pub struct EccDimmInfo {
    /// DIMM slot ID
    pub slot_id: u32,
    /// Base address
    pub base_address: u64,
    /// Size in MB
    pub size_mb: u32,
    /// ECC enabled
    pub ecc_enabled: bool,
    /// Single-bit error count
    pub single_bit_errors: u32,
    /// Multi-bit error count
    pub multi_bit_errors: u32,
    /// Last error timestamp
    pub last_error_time: u64,
}

impl EccDimmInfo {
    /// Create ECC DIMM info
    pub fn new(slot_id: u32, size_mb: u32) -> Self {
        EccDimmInfo {
            slot_id,
            base_address: 0,
            size_mb,
            ecc_enabled: false,
            single_bit_errors: 0,
            multi_bit_errors: 0,
            last_error_time: 0,
        }
    }

    /// Enable ECC
    pub fn enable_ecc(&mut self) {
        self.ecc_enabled = true;
    }

    /// Disable ECC
    pub fn disable_ecc(&mut self) {
        self.ecc_enabled = false;
    }

    /// Report single-bit error
    pub fn report_single_bit_error(&mut self) {
        self.single_bit_errors += 1;
    }

    /// Report multi-bit error
    pub fn report_multi_bit_error(&mut self) {
        self.multi_bit_errors += 1;
    }

    /// Get total errors
    pub fn get_total_errors(&self) -> u32 {
        self.single_bit_errors + self.multi_bit_errors
    }

    /// Get error rate (errors per GB)
    pub fn get_error_rate(&self) -> f64 {
        if self.size_mb == 0 {
            0.0
        } else {
            (self.get_total_errors() as f64) / (self.size_mb as f64 / 1024.0)
        }
    }
}

/// ECC manager
pub struct EccManager {
    /// ECC mode
    mode: EccMode,
    /// DIMM list
    dimms: [Option<EccDimmInfo>; 32],
    /// DIMM count
    dimm_count: usize,
    /// Error log
    error_log: [Option<EccError>; 256],
    /// Error log count
    error_log_count: usize,
    /// Total corrected errors
    total_corrected: u32,
    /// Total uncorrectable errors
    total_uncorrectable: u32,
    /// ECC initialized
    initialized: bool,
}

impl EccManager {
    /// Create ECC manager
    pub fn new() -> Self {
        EccManager {
            mode: EccMode::Disabled,
            dimms: [None; 32],
            dimm_count: 0,
            error_log: [None; 256],
            error_log_count: 0,
            total_corrected: 0,
            total_uncorrectable: 0,
            initialized: false,
        }
    }

    /// Initialize ECC
    pub fn initialize(&mut self, mode: EccMode) -> bool {
        self.mode = mode;
        self.initialized = true;

        // Enable ECC on all DIMMs
        for i in 0..self.dimm_count {
            if let Some(d) = &mut self.dimms[i] {
                if mode != EccMode::Disabled {
                    d.enable_ecc();
                }
            }
        }

        true
    }

    /// Get ECC mode
    pub fn get_mode(&self) -> EccMode {
        self.mode
    }

    /// Add DIMM
    pub fn add_dimm(&mut self, dimm: EccDimmInfo) -> bool {
        if self.dimm_count < 32 {
            self.dimms[self.dimm_count] = Some(dimm);
            self.dimm_count += 1;
            true
        } else {
            false
        }
    }

    /// Get DIMM
    pub fn get_dimm(&self, slot_id: u32) -> Option<&EccDimmInfo> {
        for i in 0..self.dimm_count {
            if let Some(d) = &self.dimms[i] {
                if d.slot_id == slot_id {
                    return Some(d);
                }
            }
        }
        None
    }

    /// Get mutable DIMM
    pub fn get_dimm_mut(&mut self, slot_id: u32) -> Option<&mut EccDimmInfo> {
        let dimm_count = self.dimm_count;
        let dimms_ptr = self.dimms.as_mut_ptr();
        
        for i in 0..dimm_count {
            unsafe {
                if let Some(d) = (*dimms_ptr.add(i)).as_mut() {
                    if d.slot_id == slot_id {
                        return Some(d);
                    }
                }
            }
        }
        None
    }

    /// Report error
    pub fn report_error(&mut self, error: EccError) -> bool {
        if self.error_log_count < 256 {
            self.error_log[self.error_log_count] = Some(error);
            self.error_log_count += 1;

            // Update statistics
            match error.error_type {
                EccErrorType::SingleBitError => {
                    self.total_corrected += 1;
                    if let Some(d) = self.get_dimm_mut(error.cpu_id) {
                        d.report_single_bit_error();
                    }
                }
                EccErrorType::MultiBitError => {
                    self.total_uncorrectable += 1;
                    if let Some(d) = self.get_dimm_mut(error.cpu_id) {
                        d.report_multi_bit_error();
                    }
                }
                EccErrorType::NoError => {}
            }

            true
        } else {
            false
        }
    }

    /// Get error from log
    pub fn get_error(&self, index: usize) -> Option<&EccError> {
        if index < self.error_log_count {
            self.error_log[index].as_ref()
        } else {
            None
        }
    }

    /// Get total corrected errors
    pub fn get_total_corrected(&self) -> u32 {
        self.total_corrected
    }

    /// Get total uncorrectable errors
    pub fn get_total_uncorrectable(&self) -> u32 {
        self.total_uncorrectable
    }

    /// Get error log count
    pub fn get_error_log_count(&self) -> usize {
        self.error_log_count
    }

    /// Get DIMM count
    pub fn get_dimm_count(&self) -> usize {
        self.dimm_count
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get DIMM with most errors
    pub fn get_worst_dimm(&self) -> Option<&EccDimmInfo> {
        let mut worst = None;
        let mut max_errors = 0;

        for i in 0..self.dimm_count {
            if let Some(d) = &self.dimms[i] {
                if d.get_total_errors() > max_errors {
                    max_errors = d.get_total_errors();
                    worst = Some(d);
                }
            }
        }

        worst
    }

    /// Disable ECC on DIMM
    pub fn disable_ecc_dimm(&mut self, slot_id: u32) -> bool {
        if let Some(dimm) = self.get_dimm_mut(slot_id) {
            dimm.disable_ecc();
            true
        } else {
            false
        }
    }

    /// Check if ECC enabled globally
    pub fn is_ecc_enabled(&self) -> bool {
        self.mode != EccMode::Disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecc_modes() {
        assert_ne!(EccMode::Correction, EccMode::DetectionOnly);
    }

    #[test]
    fn test_error_types() {
        assert_ne!(EccErrorType::SingleBitError, EccErrorType::MultiBitError);
    }

    #[test]
    fn test_ecc_error_creation() {
        let error = EccError::new(EccErrorType::SingleBitError, 0x1000);
        assert_eq!(error.error_type, EccErrorType::SingleBitError);
    }

    #[test]
    fn test_error_correctable() {
        let error = EccError::new(EccErrorType::SingleBitError, 0x1000);
        assert!(error.is_correctable());
    }

    #[test]
    fn test_error_uncorrectable() {
        let error = EccError::new(EccErrorType::MultiBitError, 0x1000);
        assert!(!error.is_correctable());
    }

    #[test]
    fn test_error_count_increment() {
        let mut error = EccError::new(EccErrorType::SingleBitError, 0x1000);
        assert_eq!(error.count, 1);
        error.increment_count();
        assert_eq!(error.count, 2);
    }

    #[test]
    fn test_ecc_dimm_creation() {
        let dimm = EccDimmInfo::new(0, 8192);
        assert_eq!(dimm.slot_id, 0);
        assert!(!dimm.ecc_enabled);
    }

    #[test]
    fn test_enable_ecc() {
        let mut dimm = EccDimmInfo::new(0, 8192);
        dimm.enable_ecc();
        assert!(dimm.ecc_enabled);
    }

    #[test]
    fn test_disable_ecc() {
        let mut dimm = EccDimmInfo::new(0, 8192);
        dimm.enable_ecc();
        dimm.disable_ecc();
        assert!(!dimm.ecc_enabled);
    }

    #[test]
    fn test_single_bit_error_report() {
        let mut dimm = EccDimmInfo::new(0, 8192);
        dimm.report_single_bit_error();
        assert_eq!(dimm.single_bit_errors, 1);
    }

    #[test]
    fn test_multi_bit_error_report() {
        let mut dimm = EccDimmInfo::new(0, 8192);
        dimm.report_multi_bit_error();
        assert_eq!(dimm.multi_bit_errors, 1);
    }

    #[test]
    fn test_total_errors() {
        let mut dimm = EccDimmInfo::new(0, 8192);
        dimm.report_single_bit_error();
        dimm.report_multi_bit_error();
        assert_eq!(dimm.get_total_errors(), 2);
    }

    #[test]
    fn test_error_rate() {
        let mut dimm = EccDimmInfo::new(0, 8192);
        dimm.report_single_bit_error();
        let rate = dimm.get_error_rate();
        assert!(rate > 0.0);
    }

    #[test]
    fn test_manager_creation() {
        let mgr = EccManager::new();
        assert_eq!(mgr.get_mode(), EccMode::Disabled);
        assert!(!mgr.is_initialized());
    }

    #[test]
    fn test_initialize_ecc() {
        let mut mgr = EccManager::new();
        assert!(mgr.initialize(EccMode::Correction));
        assert!(mgr.is_initialized());
    }

    #[test]
    fn test_add_dimm() {
        let mut mgr = EccManager::new();
        let dimm = EccDimmInfo::new(0, 8192);
        assert!(mgr.add_dimm(dimm));
    }

    #[test]
    fn test_get_dimm() {
        let mut mgr = EccManager::new();
        let dimm = EccDimmInfo::new(0, 8192);
        mgr.add_dimm(dimm);
        assert!(mgr.get_dimm(0).is_some());
    }

    #[test]
    fn test_report_error() {
        let mut mgr = EccManager::new();
        mgr.add_dimm(EccDimmInfo::new(0, 8192));
        let error = EccError::new(EccErrorType::SingleBitError, 0x1000);
        assert!(mgr.report_error(error));
    }

    #[test]
    fn test_error_log() {
        let mut mgr = EccManager::new();
        mgr.add_dimm(EccDimmInfo::new(0, 8192));
        let error = EccError::new(EccErrorType::SingleBitError, 0x1000);
        mgr.report_error(error);
        assert!(mgr.get_error(0).is_some());
    }

    #[test]
    fn test_total_corrected_errors() {
        let mut mgr = EccManager::new();
        mgr.add_dimm(EccDimmInfo::new(0, 8192));
        let error = EccError::new(EccErrorType::SingleBitError, 0x1000);
        mgr.report_error(error);
        assert_eq!(mgr.get_total_corrected(), 1);
    }

    #[test]
    fn test_total_uncorrectable_errors() {
        let mut mgr = EccManager::new();
        mgr.add_dimm(EccDimmInfo::new(0, 8192));
        let error = EccError::new(EccErrorType::MultiBitError, 0x1000);
        mgr.report_error(error);
        assert_eq!(mgr.get_total_uncorrectable(), 1);
    }

    #[test]
    fn test_get_worst_dimm() {
        let mut mgr = EccManager::new();
        let dimm1 = EccDimmInfo::new(0, 8192);
        let mut dimm2 = EccDimmInfo::new(1, 8192);
        dimm2.report_multi_bit_error();
        dimm2.report_multi_bit_error();
        mgr.add_dimm(dimm1);
        mgr.add_dimm(dimm2);
        let worst = mgr.get_worst_dimm();
        assert!(worst.is_some());
    }

    #[test]
    fn test_disable_ecc_dimm() {
        let mut mgr = EccManager::new();
        let mut dimm = EccDimmInfo::new(0, 8192);
        dimm.enable_ecc();
        mgr.add_dimm(dimm);
        assert!(mgr.disable_ecc_dimm(0));
    }

    #[test]
    fn test_is_ecc_enabled() {
        let mut mgr = EccManager::new();
        assert!(!mgr.is_ecc_enabled());
        mgr.initialize(EccMode::Correction);
        assert!(mgr.is_ecc_enabled());
    }

    #[test]
    fn test_multiple_dimms_ecc() {
        let mut mgr = EccManager::new();
        for i in 0..4 {
            mgr.add_dimm(EccDimmInfo::new(i, 4096));
        }
        mgr.initialize(EccMode::Correction);
        assert_eq!(mgr.get_dimm_count(), 4);
    }

    #[test]
    fn test_error_log_size() {
        let mut mgr = EccManager::new();
        mgr.add_dimm(EccDimmInfo::new(0, 8192));
        for i in 0..10 {
            let error = EccError::new(EccErrorType::SingleBitError, 0x1000 + (i * 0x1000) as u64);
            mgr.report_error(error);
        }
        assert_eq!(mgr.get_error_log_count(), 10);
    }

    #[test]
    fn test_ecc_detection_mode() {
        let mut mgr = EccManager::new();
        mgr.initialize(EccMode::DetectionOnly);
        assert_eq!(mgr.get_mode(), EccMode::DetectionOnly);
    }

    #[test]
    fn test_error_with_syndrome() {
        let mut error = EccError::new(EccErrorType::SingleBitError, 0x1000);
        error.syndrome = 0x42;
        assert_eq!(error.syndrome, 0x42);
    }

    #[test]
    fn test_get_dimm_mut() {
        let mut mgr = EccManager::new();
        let dimm = EccDimmInfo::new(0, 8192);
        mgr.add_dimm(dimm);
        if let Some(d) = mgr.get_dimm_mut(0) {
            d.enable_ecc();
        }
        assert!(mgr.get_dimm(0).unwrap().ecc_enabled);
    }

    #[test]
    fn test_large_error_log() {
        let mut mgr = EccManager::new();
        mgr.add_dimm(EccDimmInfo::new(0, 8192));
        for _ in 0..256 {
            let error = EccError::new(EccErrorType::SingleBitError, 0x1000);
            if !mgr.report_error(error) {
                break;
            }
        }
        assert!(mgr.get_error_log_count() <= 256);
    }

    #[test]
    fn test_dimm_base_address() {
        let mut dimm = EccDimmInfo::new(0, 8192);
        dimm.base_address = 0x80000000;
        assert_eq!(dimm.base_address, 0x80000000);
    }

    #[test]
    fn test_error_cpu_id() {
        let mut error = EccError::new(EccErrorType::SingleBitError, 0x1000);
        error.cpu_id = 2;
        assert_eq!(error.cpu_id, 2);
    }
}
