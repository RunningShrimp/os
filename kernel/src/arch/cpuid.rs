//! CPUID (CPU Identification) support
//!
//! This module provides functions to read CPU identification information
//! using the CPUID instruction on x86_64 architectures.

/// Get the current CPU ID (simplified implementation)
///
/// In a real implementation, this would use CPU-local storage
/// or architecture-specific registers to get the actual CPU ID.
pub fn cpuid() -> u32 {
    // Simplified implementation - returns 0 for now
    // In a real kernel, this would use per-CPU data structures
    0
}

/// Get the current CPU ID as a usize
pub fn cpu_id() -> usize {
    cpuid() as usize
}

/// CPUID result structure
#[derive(Debug, Clone, Copy)]
pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

/// Execute CPUID instruction (x86_64 only)
#[cfg(target_arch = "x86_64")]
pub unsafe fn cpuid_raw(eax: u32, ecx: u32) -> CpuidResult {
    let mut result = CpuidResult {
        eax: 0,
        ebx: 0,
        ecx: 0,
        edx: 0,
    };

    core::arch::asm!(
        "cpuid",
        inlateout("eax") eax,
        inlateout("ecx") ecx => _,
        lateout("ebx") result.ebx,
        lateout("edx") result.edx,
        options(nostack, nomem, preserves_flags)
    );

    result
}

/// Check if CPU supports a specific feature
///
/// The `feature` parameter should be a CPU feature bit number.
/// For x86_64, this checks against CPUID leaf 1 EDX register.
/// Common features: 0-31 are in EDX, 32-63 would be in ECX.
pub fn has_feature(feature: u32) -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            let result = cpuid_raw(1, 0);
            // Map feature bit to appropriate register
            let bit = feature % 32;
            if feature < 32 {
                (result.edx & (1 << bit)) != 0
            } else {
                // TODO: Check ECX register for features 32-63
                false
            }
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        // TODO: Implement feature detection for other architectures
        let _ = feature; // Acknowledge parameter
        false
    }
}

/// Get CPU vendor string
pub fn get_vendor_string() -> alloc::string::String {
    #[cfg(target_arch = "x86_64")]
    {
        extern crate alloc;
        use alloc::string::String;

        unsafe {
            let result = cpuid_raw(0, 0);
            
            let mut vendor = [0u8; 12];
            vendor[0..4].copy_from_slice(&result.ebx.to_le_bytes());
            vendor[4..8].copy_from_slice(&result.edx.to_le_bytes());
            vendor[8..12].copy_from_slice(&result.ecx.to_le_bytes());
            
            String::from_utf8_lossy(&vendor).to_string()
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        extern crate alloc;
        use alloc::string::String;
        String::from("Unknown")
    }
}

/// Get maximum CPUID leaf
pub fn get_max_leaf() -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            let result = cpuid_raw(0, 0);
            result.eax
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

/// Get CPU brand string
pub fn get_brand_string() -> alloc::string::String {
    #[cfg(target_arch = "x86_64")]
    {
        extern crate alloc;

        unsafe {
            let mut brand = [0u8; 48];
            
            // CPUID leaf 0x80000002, 0x80000003, 0x80000004 contain the brand string
            for (i, leaf) in [0x80000002u32, 0x80000003, 0x80000004].iter().enumerate() {
                let result = cpuid_raw(*leaf, 0);
                let offset = i * 16;
                brand[offset..offset+4].copy_from_slice(&result.eax.to_le_bytes());
                brand[offset+4..offset+8].copy_from_slice(&result.ebx.to_le_bytes());
                brand[offset+8..offset+12].copy_from_slice(&result.ecx.to_le_bytes());
                brand[offset+12..offset+16].copy_from_slice(&result.edx.to_le_bytes());
            }
            
            // Trim null bytes and convert to string
            let len = brand.iter().position(|&b| b == 0).unwrap_or(48);
            String::from_utf8_lossy(&brand[..len]).to_string()
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        extern crate alloc;
        use alloc::string::String;
        String::from("Unknown")
    }
}
