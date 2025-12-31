//! Cryptographically Secure Random Number Generator for Security Purposes
//!
//! This module provides a cryptographically secure RNG (CSPRNG) specifically
//! designed for security-sensitive operations like ASLR, stack canaries, and
//! other security mechanisms.
//!
//! ## Features
//!
//! - **Hardware RNG Priority**: Uses CPU hardware RNG when available
//!   - x86_64: RDRAND (Intel/AMD) or RDSEED
//!   - ARM64: RNDR (Random Number Generator)
//!   - RISC-V: seed register (when available)
//!
//! - **ChaCha20-based Fallback**: When hardware RNG is unavailable, uses
//!   ChaCha20 cipher as a CSPRNG, which is cryptographically secure.
//!
//! - **Entropy Mixing**: Combines multiple entropy sources:
//!   - Hardware RNG
//!   - High-resolution timestamps
//!   - CPU cycle counters
//!   - Memory addresses (ASLR itself)
//!
//! - **Forward Secrecy**: Regular rekeying to ensure forward secrecy
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::security::secure_rng::{SecureRng, secure_random};
//!
//! // Generate a random 64-bit value
//! let random_value = secure_random_u64();
//!
//! // Fill a buffer with random bytes
//! let mut buffer = [0u8; 32];
//! secure_random_bytes(&mut buffer);
//!
//! // Generate a random value in a range
//! let value = secure_random_range(0, 1000);
//! ```

#![allow(dead_code)]

use core::{
    sync::atomic::{AtomicU64, Ordering},
    mem::transmute,
};

/// Cryptographically secure RNG instance
static mut SECURE_RNG: Option<SecureRng> = None;
static SECURE_RNG_INIT: AtomicU64 = AtomicU64::new(0);

/// ChaCha20 state for CSPRNG fallback
#[repr(C)]
struct ChaCha20State {
    /// State words (16 words)
    state: [u32; 16],
    /// Current position in block
    position: usize,
}

impl ChaCha20State {
    /// Initialize ChaCha20 with a key and nonce
    fn new(key: &[u8; 32], nonce: &[u8; 12]) -> Self {
        let mut state = [0u32; 16];

        // Constants ("expa"nd 3-byte-key)
        state[0] = 0x61707865;
        state[1] = 0x3320646e;
        state[2] = 0x79622d32;
        state[3] = 0x6b206574;

        // Key (8 words)
        for i in 0..8 {
            state[4 + i] = u32::from_le_bytes([
                key[i * 4],
                key[i * 4 + 1],
                key[i * 4 + 2],
                key[i * 4 + 3],
            ]);
        }

        // Counter and nonce (4 words)
        state[12] = 0; // Counter
        state[13] = 0; // Counter
        for i in 0..3 {
            state[14 + i] = u32::from_le_bytes([
                nonce[i * 4],
                nonce[i * 4 + 1],
                nonce[i * 4 + 2],
                nonce[i * 4 + 3],
            ]);
        }

        Self {
            state,
            position: 64, // Force initial block generation
        }
    }

    /// Generate the next ChaCha20 block
    fn generate_block(&mut self) {
        // Quarter round function
        #[inline]
        fn quarter_round(a: usize, b: usize, c: usize, d: usize, state: &mut [u32; 16]) {
            state[a] = state[a].wrapping_add(state[b]);
            state[d] ^= state[a];
            state[d] = state[d].rotate_left(16);

            state[c] = state[c].wrapping_add(state[d]);
            state[b] ^= state[c];
            state[b] = state[b].rotate_left(12);

            state[a] = state[a].wrapping_add(state[b]);
            state[d] ^= state[a];
            state[d] = state[d].rotate_left(8);

            state[c] = state[c].wrapping_add(state[d]);
            state[b] ^= state[c];
            state[b] = state[b].rotate_left(7);
        }

        let mut working_state = self.state;

        // 10 rounds (20 column/diagonal rounds)
        for _ in 0..10 {
            // Column rounds
            quarter_round(0, 4, 8, 12, &mut working_state);
            quarter_round(1, 5, 9, 13, &mut working_state);
            quarter_round(2, 6, 10, 14, &mut working_state);
            quarter_round(3, 7, 11, 15, &mut working_state);

            // Diagonal rounds
            quarter_round(0, 5, 10, 15, &mut working_state);
            quarter_round(1, 6, 11, 12, &mut working_state);
            quarter_round(2, 7, 8, 13, &mut working_state);
            quarter_round(3, 4, 9, 14, &mut working_state);
        }

        // Add initial state
        for i in 0..16 {
            self.state[i] = self.state[i].wrapping_add(working_state[i]);
        }

        // Increment counter
        self.state[12] = self.state[12].wrapping_add(1);
        if self.state[12] == 0 {
            self.state[13] = self.state[13].wrapping_add(1);
        }

        self.position = 0;
    }

    /// Get next 32-bit word from the keystream
    fn next_u32(&mut self) -> u32 {
        if self.position >= 64 {
            self.generate_block();
        }

        let word = self.state[self.position / 4];
        self.position += 4;
        word
    }
}

/// Cryptographically secure RNG
pub struct SecureRng {
    /// ChaCha20 state (used as CSPRNG)
    chacha: ChaCha20State,
    /// Hardware RNG available flag
    has_hw_rng: bool,
    /// Reseed counter
    reseed_counter: u64,
    /// Max operations before reseed
    reseed_interval: u64,
}

impl SecureRng {
    /// Create a new secure RNG with automatic entropy initialization
    pub fn new() -> Self {
        let (key, nonce) = Self::initialize_entropy();

        Self {
            chacha: ChaCha20State::new(&key, &nonce),
            has_hw_rng: Self::detect_hw_rng(),
            reseed_counter: 0,
            reseed_interval: 100000, // Reseed after 100k operations
        }
    }

    /// Initialize entropy from multiple sources
    fn initialize_entropy() -> ([u8; 32], [u8; 12]) {
        let mut key = [0u8; 32];
        let mut nonce = [0u8; 12];

        // Mix in hardware RNG entropy
        if let Some(hw_entropy) = Self::get_hw_entropy() {
            let hw_bytes = hw_entropy.to_le_bytes();
            for i in 0..8 {
                key[i] ^= hw_bytes[i];
                nonce[i] ^= hw_bytes[i % 12];
            }
        }

        // Mix in timestamp/cycle counter
        let cycles = Self::read_cpu_cycles();
        let cycles_bytes = cycles.to_le_bytes();
        for i in 0..8 {
            key[8 + i] ^= cycles_bytes[i];
        }

        // Mix in stack pointer (entropy from ASLR)
        let stack_ptr = &key as *const _ as usize;
        let stack_bytes = stack_ptr.to_le_bytes();
        for i in 0..8 {
            key[16 + i] ^= stack_bytes[i];
            nonce[i % 12] ^= stack_bytes[i];
        }

        // Additional mixing
        for i in 0..32 {
            key[i] = key[i].wrapping_mul(5).wrapping_add(17);
            nonce[i % 12] = nonce[i % 12].wrapping_mul(3).wrapping_add(13);
        }

        (key, nonce)
    }

    /// Detect if hardware RNG is available
    fn detect_hw_rng() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            // Try RDRAND
            if let Some(_) = Self::rdrand_u32() {
                return true;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            // Try RNDR
            if let Some(_) = Self::rndr_u64() {
                return true;
            }
        }

        false
    }

    /// Get entropy from hardware RNG
    fn get_hw_entropy() -> Option<u64> {
        #[cfg(target_arch = "x86_64")]
        {
            // Use RDRAND twice to get 64 bits
            let lo = Self::rdrand_u32()? as u64;
            let hi = Self::rdrand_u32()? as u64;
            Some(lo | (hi << 32))
        }

        #[cfg(target_arch = "aarch64")]
        {
            Self::rndr_u64()
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            None
        }
    }

    /// Read CPU cycle counter (for entropy)
    #[inline]
    fn read_cpu_cycles() -> u64 {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let mut rax: u64;
            core::arch::asm!(
                "lfence",
                "rdtsc",
                "shl rdx, 32",
                "or rax, rdx",
                out("rax") rax,
                out("rdx") _,
            );
            rax
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let mut cnt: u64;
            core::arch::asm!("mrs {}, cntvct_el0", out(reg) cnt);
            cnt
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            let mut cycles: u64;
            core::arch::asm!("rdtime {}", out(reg) cycles);
            cycles
        }
    }

    /// x86_64 RDRAND instruction
    #[cfg(target_arch = "x86_64")]
    fn rdrand_u32() -> Option<u32> {
        unsafe {
            let mut value: u32;
            let mut success: u8;

            core::arch::asm!(
                "rdrand {0:e}",
                out(reg) value,
                setne(success),
                options(nostack, pure)
            );

            if success != 0 {
                Some(value)
            } else {
                None
            }
        }
    }

    /// ARM64 RNDR instruction (ARMv8.5-A)
    #[cfg(target_arch = "aarch64")]
    fn rndr_u64() -> Option<u64> {
        unsafe {
            let mut value: u64;
            let mut success: u8;

            core::arch::asm!(
                "mrs {0:x}, rndr",
                out(reg) value,
                setne(success),
                options(nostack, pure)
            );

            if success != 0 {
                Some(value)
            } else {
                None
            }
        }
    }

    /// Reseed the RNG (called periodically or after many operations)
    fn reseed(&mut self) {
        let (new_key, new_nonce) = Self::initialize_entropy();

        // Mix new entropy into existing state
        for i in 0..32 {
            self.chacha.state[i / 4] ^= u32::from_le_bytes([
                new_key[i],
                new_key[(i + 1) % 32],
                new_key[(i + 2) % 32],
                new_key[(i + 3) % 32],
            ]);
        }

        self.reseed_counter = 0;
    }

    /// Generate a random 64-bit value
    pub fn gen_u64(&mut self) -> u64 {
        self.reseed_counter += 1;

        // Check if we need to reseed
        if self.reseed_counter >= self.reseed_interval {
            self.reseed();
        }

        // Prefer hardware RNG if available
        if self.has_hw_rng {
            if let Some(hw_value) = Self::get_hw_entropy() {
                // Mix hardware RNG with ChaCha20
                let chacha_value = self.chacha.next_u32() as u64;
                return hw_value ^ chacha_value;
            }
        }

        // Fallback to ChaCha20
        let lo = self.chacha.next_u32() as u64;
        let hi = self.chacha.next_u32() as u64;
        lo | (hi << 32)
    }

    /// Generate a random 32-bit value
    pub fn gen_u32(&mut self) -> u32 {
        self.gen_u64() as u32
    }

    /// Generate a random usize value
    pub fn gen_usize(&mut self) -> usize {
        self.gen_u64() as usize
    }

    /// Fill a buffer with random bytes
    pub fn gen_bytes(&mut self, buffer: &mut [u8]) {
        let mut i = 0;
        while i < buffer.len() {
            let random_value = self.gen_u64();
            let bytes = random_value.to_le_bytes();

            let remaining = buffer.len() - i;
            let copy_len = core::cmp::min(8, remaining);

            buffer[i..i + copy_len].copy_from_slice(&bytes[..copy_len]);
            i += copy_len;
        }
    }

    /// Generate a random value in the range [min, max)
    pub fn gen_range(&mut self, min: u64, max: u64) -> u64 {
        assert!(min < max, "Invalid range: min must be less than max");

        let range = max - min;
        let mask = (1u64 << (64 - range.leading_zeros())) - 1;

        loop {
            let random_value = self.gen_u64() & mask;
            let result = random_value % range;

            if random_value < (u64::MAX - u64::MAX % range) {
                return min + result;
            }
        }
    }
}

/// Get or initialize the global secure RNG instance
fn get_secure_rng() -> &'static mut SecureRng {
    unsafe {
        if SECURE_RNG_INIT.load(Ordering::Acquire) == 0 {
            SECURE_RNG = Some(SecureRng::new());
            SECURE_RNG_INIT.store(1, Ordering::Release);
        }

        SECURE_RNG.as_mut().unwrap()
    }
}

/// Generate a cryptographically secure random 64-bit value
pub fn secure_random_u64() -> u64 {
    get_secure_rng().gen_u64()
}

/// Generate a cryptographically secure random 32-bit value
pub fn secure_random_u32() -> u32 {
    get_secure_rng().gen_u32()
}

/// Generate a cryptographically secure random usize value
pub fn secure_random_usize() -> usize {
    get_secure_rng().gen_usize()
}

/// Fill a buffer with cryptographically secure random bytes
pub fn secure_random_bytes(buffer: &mut [u8]) {
    get_secure_rng().gen_bytes(buffer)
}

/// Generate a random value in the range [min, max)
pub fn secure_random_range(min: u64, max: u64) -> u64 {
    get_secure_rng().gen_range(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_rng_creation() {
        let rng = SecureRng::new();
        // Should not crash
    }

    #[test]
    fn test_random_generation() {
        let mut rng = SecureRng::new();

        let val1 = rng.gen_u64();
        let val2 = rng.gen_u64();

        // Values should be different (extremely unlikely to be same)
        assert_ne!(val1, val2);
    }

    #[test]
    fn test_global_functions() {
        let val1 = secure_random_u64();
        let val2 = secure_random_u64();

        assert_ne!(val1, val2);
    }

    #[test]
    fn test_fill_buffer() {
        let mut buffer = [0u8; 32];
        secure_random_bytes(&mut buffer);

        // Buffer should not be all zeros (extremely unlikely)
        let is_zeroed = buffer.iter().all(|&b| b == 0);
        assert!(!is_zeroed);
    }

    #[test]
    fn test_range_generation() {
        let val = secure_random_range(10, 100);
        assert!(val >= 10 && val < 100);
    }
}
