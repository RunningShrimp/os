//! Digital Signal Processing Engine for Satellite Communications
//!
//! This module provides high-performance DSP primitives for satellite communication
//! signal processing, including FFT, filtering, synchronization, and demodulation.
//!
//! # Features
//! - Cooley-Tukey FFT/IFFT with radix-2/radix-4 optimizations
//! - FIR/IIR filtering with various window functions
//! - Digital down/up conversion
//! - Adaptive equalization
//! - Carrier and symbol timing synchronization
//! - Soft-decision demodulation

#![allow(dead_code)]

extern crate alloc;

use alloc::string::ToString;
use alloc::vec::Vec;
use core::f64::consts::PI;

// Use libm for mathematical functions in no_std environment
use libm::{cos, sin, sqrt, exp, atan2, log2};

use super::sat_types::{ModulationScheme, SatComError, SatResult};

// ============================================================================
// Complex Number Operations
// ============================================================================

/// Complex number representation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    /// Create new complex number
    #[inline]
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// Create from polar coordinates
    #[inline]
    pub fn from_polar(mag: f64, phase: f64) -> Self {
        Self {
            re: mag * cos(phase),
            im: mag * sin(phase),
        }
    }

    /// Get magnitude
    #[inline]
    pub fn mag(&self) -> f64 {
        sqrt(self.re * self.re + self.im * self.im)
    }

    /// Get magnitude squared (more efficient)
    #[inline]
    pub fn mag_sq(&self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// Get phase in radians
    #[inline]
    pub fn phase(&self) -> f64 {
        atan2(self.im, self.re)
    }

    /// Get complex conjugate
    #[inline]
    pub fn conj(&self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    /// Complex exponential
    #[inline]
    pub fn exp(&self) -> Self {
        let ea = exp(self.re);
        Self {
            re: ea * cos(self.im),
            im: ea * sin(self.im),
        }
    }

    /// Natural logarithm
    #[inline]
    pub fn ln(&self) -> Self {
        Self {
            re: libm::log(self.mag()),
            im: self.phase(),
        }
    }
}

impl core::ops::Add for Complex {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl core::ops::Sub for Complex {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

impl core::ops::Mul for Complex {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl core::ops::Mul<f64> for Complex {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f64) -> Self {
        Self {
            re: self.re * scalar,
            im: self.im * scalar,
        }
    }
}

// ============================================================================
// Fast Fourier Transform
// ============================================================================

/// FFT configuration and computation
pub struct FftEngine {
    /// FFT size (must be power of 2)
    size: usize,

    /// Pre-computed twiddle factors
    twiddles: Vec<Complex>,

    /// Bit-reversal table
    bit_reverse: Vec<usize>,
}

impl FftEngine {
    /// Create new FFT engine
    pub fn new(size: usize) -> SatResult<Self> {
        // Verify size is power of 2
        if !size.is_power_of_two() {
            return Err(SatComError::InvalidParameter(
                "FFT size must be power of 2".to_string(),
            ));
        }

        let mut twiddles = Vec::with_capacity(size / 2);
        for k in 0..(size / 2) {
            let angle = -2.0 * PI * k as f64 / size as f64;
            twiddles.push(Complex::from_polar(1.0, angle));
        }

        let bit_reverse = Self::compute_bit_reverse(size);

        Ok(Self {
            size,
            twiddles,
            bit_reverse,
        })
    }

    /// Compute bit-reversal permutation table
    fn compute_bit_reverse(size: usize) -> Vec<usize> {
        let mut table = Vec::with_capacity(size);
        let n_bits = log2(size as f64) as usize;

        for i in 0..size {
            let mut reversed = 0;
            for j in 0..n_bits {
                if (i >> j) & 1 != 0 {
                    reversed |= 1 << (n_bits - 1 - j);
                }
            }
            table.push(reversed);
        }

        table
    }

    /// Perform in-place FFT (decimation-in-time)
    pub fn fft(&self, data: &mut [Complex]) -> SatResult<()> {
        if data.len() != self.size {
            return Err(SatComError::InvalidParameter(
                "Data length must match FFT size".to_string(),
            ));
        }

        // Bit-reversal reordering
        for i in 0..self.size {
            let j = self.bit_reverse[i];
            if i < j {
                data.swap(i, j);
            }
        }

        // Cooley-Tukey butterfly operations
        let mut len = 2;
        while len <= self.size {
            let half_len = len / 2;
            let step = self.size / len;

            for i in (0..self.size).step_by(len) {
                for j in 0..half_len {
                    let idx = i + j;
                    let twiddle = self.twiddles[j * step];
                    let temp = twiddle * data[idx + half_len];

                    data[idx + half_len] = data[idx] - temp;
                    data[idx] = data[idx] + temp;
                }
            }

            len <<= 1;
        }

        Ok(())
    }

    /// Perform in-place IFFT
    pub fn ifft(&self, data: &mut [Complex]) -> SatResult<()> {
        // Conjugate input
        for sample in data.iter_mut() {
            sample.im = -sample.im;
        }

        // Perform FFT
        self.fft(data)?;

        // Conjugate and scale output
        let scale = 1.0 / self.size as f64;
        for sample in data.iter_mut() {
            sample.im = -sample.im;
            *sample = *sample * scale;
        }

        Ok(())
    }

    /// Compute FFT of real-valued input (optimized)
    pub fn fft_real(&self, input: &[f64]) -> SatResult<Vec<Complex>> {
        if input.len() != self.size {
            return Err(SatComError::InvalidParameter(
                "Input length must match FFT size".to_string(),
            ));
        }

        // Pack real data as complex
        let mut complex_data: Vec<Complex> = input.iter().map(|&x| Complex::new(x, 0.0)).collect();

        self.fft(&mut complex_data)?;

        Ok(complex_data)
    }

    /// Compute power spectrum
    pub fn power_spectrum(&self, data: &[f64]) -> SatResult<Vec<f64>> {
        let spectrum = self.fft_real(data)?;

        Ok(spectrum.iter().map(|c| c.mag_sq()).collect())
    }

    /// Perform cross-correlation using FFT
    pub fn correlate(&self, signal: &[f64], pattern: &[f64]) -> SatResult<Vec<f64>> {
        if signal.len() != self.size || pattern.len() != self.size {
            return Err(SatComError::InvalidParameter(
                "Signal and pattern must match FFT size".to_string(),
            ));
        }

        // FFT of both signals
        let mut signal_fft: Vec<Complex> =
            signal.iter().map(|&x| Complex::new(x, 0.0)).collect();
        let mut pattern_fft: Vec<Complex> =
            pattern.iter().map(|&x| Complex::new(x, 0.0)).collect();

        self.fft(&mut signal_fft)?;
        self.fft(&mut pattern_fft)?;

        // Multiply in frequency domain
        for i in 0..self.size {
            signal_fft[i] = signal_fft[i] * pattern_fft[i].conj();
        }

        // IFFT back to time domain
        self.ifft(&mut signal_fft)?;

        Ok(signal_fft.iter().map(|c| c.re).collect())
    }
}

// ============================================================================
// Digital Filters
// ============================================================================

/// FIR filter structure
pub struct FirFilter {
    /// Filter coefficients
    coefficients: Vec<f64>,

    /// Delay line (state)
    state: Vec<f64>,

    /// Current position in circular buffer
    pos: usize,
}

impl FirFilter {
    /// Create new FIR filter from coefficients
    pub fn new(coefficients: Vec<f64>) -> Self {
        let n_taps = coefficients.len();
        let mut state = Vec::with_capacity(n_taps);
        state.resize(n_taps, 0.0);

        Self {
            coefficients,
            state,
            pos: 0,
        }
    }

    /// Create low-pass filter using window method
    pub fn low_pass(cutoff: f64, sample_rate: f64, n_taps: usize, window_type: WindowType) -> Self {
        let mut coeffs = Vec::with_capacity(n_taps);
        let center = (n_taps - 1) as f64 / 2.0;
        let norm_cutoff = cutoff / sample_rate;

        for n in 0..n_taps {
            let t = n as f64 - center;
            let mut coeff;

            // Sinc function
            if t == 0.0 {
                coeff = 2.0 * norm_cutoff;
            } else {
                let arg = 2.0 * PI * norm_cutoff * t;
                coeff = sin(arg) / (PI * t);
            }

            // Apply window
            coeff *= window_type.value(n, n_taps);

            coeffs.push(coeff);
        }

        // Normalize
        let sum: f64 = coeffs.iter().sum();
        for coeff in coeffs.iter_mut() {
            *coeff /= sum;
        }

        Self::new(coeffs)
    }

    /// Create high-pass filter
    pub fn high_pass(cutoff: f64, sample_rate: f64, n_taps: usize, window_type: WindowType) -> Self {
        let mut coeffs = Vec::with_capacity(n_taps);
        let center = (n_taps - 1) as f64 / 2.0;
        let norm_cutoff = cutoff / sample_rate;

        for n in 0..n_taps {
            let t = n as f64 - center;
            let mut coeff;

            // High-pass sinc
            if t == 0.0 {
                coeff = 1.0 - 2.0 * norm_cutoff;
            } else {
                let arg = 2.0 * PI * norm_cutoff * t;
                coeff = -sin(arg) / (PI * t);
            }

            // Apply window
            coeff *= window_type.value(n, n_taps);

            coeffs.push(coeff);
        }

        // Normalize
        let sum: f64 = coeffs.iter().map(|&c| c.abs()).sum();
        for coeff in coeffs.iter_mut() {
            *coeff /= sum;
        }

        Self::new(coeffs)
    }

    /// Process single sample
    pub fn filter(&mut self, input: f64) -> f64 {
        // Insert new sample
        self.state[self.pos] = input;
        self.pos = (self.pos + 1) % self.coefficients.len();

        // Compute convolution
        let mut output = 0.0;
        let n_taps = self.coefficients.len();

        for i in 0..n_taps {
            let idx = (self.pos + n_taps - 1 - i) % n_taps;
            output += self.state[idx] * self.coefficients[i];
        }

        output
    }

    /// Process buffer of samples
    pub fn filter_buffer(&mut self, input: &[f64], output: &mut [f64]) {
        for (i, &sample) in input.iter().enumerate() {
            output[i] = self.filter(sample);
        }
    }
}

/// Window function types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    Rectangular,
    Hamming,
    Hann,
    Blackman,
    BlackmanHarris,
}

impl WindowType {
    /// Get window value at index n for length N
    pub fn value(&self, n: usize, n_total: usize) -> f64 {
        let n = n as f64;
        let n_total = n_total as f64;
        let norm = n / (n_total - 1.0);

        match self {
            WindowType::Rectangular => 1.0,

            WindowType::Hamming => {
                0.54 - 0.46 * cos(2.0 * PI * norm)
            }

            WindowType::Hann => {
                0.5 - 0.5 * cos(2.0 * PI * norm)
            }

            WindowType::Blackman => {
                0.42 - 0.5 * cos(2.0 * PI * norm) + 0.08 * cos(4.0 * PI * norm)
            }

            WindowType::BlackmanHarris => {
                0.35875 - 0.48829 * cos(2.0 * PI * norm)
                    + 0.14128 * cos(4.0 * PI * norm)
                    - 0.01168 * cos(6.0 * PI * norm)
            }
        }
    }
}

/// IIR filter structure (biquad)
pub struct IirBiquad {
    /// Numerator coefficients (b0, b1, b2)
    b: [f64; 3],

    /// Denominator coefficients (a1, a2), a0 = 1
    a: [f64; 2],

    /// State variables
    w1: f64,
    w2: f64,
}

impl IirBiquad {
    /// Create new biquad filter
    pub fn new(b: [f64; 3], a: [f64; 2]) -> Self {
        Self { b, a, w1: 0.0, w2: 0.0 }
    }

    /// Create low-pass biquad (butterworth)
    pub fn low_pass(sample_rate: f64, cutoff: f64, q: f64) -> Self {
        let omega = 2.0 * PI * cutoff / sample_rate;
        let alpha = sin(omega) / (2.0 * q);
        let cos_omega = cos(omega);

        let b0 = (1.0 - cos_omega) / 2.0;
        let b1 = 1.0 - cos_omega;
        let b2 = (1.0 - cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self::new(
            [b0 / a0, b1 / a0, b2 / a0],
            [a1 / a0, a2 / a0],
        )
    }

    /// Process single sample
    pub fn filter(&mut self, input: f64) -> f64 {
        // Direct form II transposed
        let output = self.b[0] * input + self.w1;
        self.w1 = self.b[1] * input - self.a[0] * output + self.w2;
        self.w2 = self.b[2] * input - self.a[1] * output;

        output
    }
}

// ============================================================================
// Digital Down/Up Conversion
// ============================================================================

/// Digital down-converter (mix to baseband)
pub struct DigitalDownConverter {
    /// Local oscillator frequency (normalized to sample rate)
    lo_freq: f64,

    /// Current phase accumulator
    phase: f64,

    /// Phase increment per sample
    phase_inc: f64,
}

impl DigitalDownConverter {
    /// Create new DDC
    pub fn new(lo_freq: f64, sample_rate: f64) -> Self {
        let phase_inc = 2.0 * PI * lo_freq / sample_rate;

        Self {
            lo_freq,
            phase: 0.0,
            phase_inc,
        }
    }

    /// Mix single sample to baseband (I/Q output)
    pub fn mix(&mut self, input: f64) -> (f64, f64) {
        let i = input * cos(self.phase);
        let q = input * sin(self.phase);

        // Advance phase
        self.phase = (self.phase + self.phase_inc) % (2.0 * PI);

        (i, q)
    }

    /// Mix buffer of samples
    pub fn mix_buffer(&mut self, input: &[f64], i_output: &mut [f64], q_output: &mut [f64]) {
        for (idx, &sample) in input.iter().enumerate() {
            let (i, q) = self.mix(sample);
            i_output[idx] = i;
            q_output[idx] = q;
        }
    }

    /// Reset phase
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }
}

/// Digital up-converter (mix from baseband)
pub struct DigitalUpConverter {
    /// Local oscillator frequency (normalized)
    lo_freq: f64,

    /// Current phase accumulator
    phase: f64,

    /// Phase increment per sample
    phase_inc: f64,
}

impl DigitalUpConverter {
    /// Create new DUC
    pub fn new(lo_freq: f64, sample_rate: f64) -> Self {
        let phase_inc = 2.0 * PI * lo_freq / sample_rate;

        Self {
            lo_freq,
            phase: 0.0,
            phase_inc,
        }
    }

    /// Mix I/Q samples to RF
    pub fn mix(&mut self, i: f64, q: f64) -> f64 {
        let output = i * cos(self.phase) - q * sin(self.phase);

        // Advance phase
        self.phase = (self.phase + self.phase_inc) % (2.0 * PI);

        output
    }

    /// Reset phase
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }
}

// ============================================================================
// Adaptive Equalizer
// ============================================================================

/// LMS adaptive equalizer
pub struct LmsEqualizer {
    /// Filter coefficients
    coefficients: Vec<Complex>,

    /// Step size (adaptation rate)
    step_size: f64,

    /// Input delay line
    delay_line: Vec<Complex>,
}

impl LmsEqualizer {
    /// Create new equalizer
    pub fn new(n_taps: usize, step_size: f64) -> Self {
        Self {
            coefficients: vec![Complex::new(0.0, 0.0); n_taps],
            step_size,
            delay_line: vec![Complex::new(0.0, 0.0); n_taps],
        }
    }

    /// Process sample and update coefficients
    pub fn equalize(&mut self, input: Complex, reference: Complex) -> Complex {
        // Shift delay line
        self.delay_line.rotate_right(1);
        self.delay_line[0] = input;

        // Compute output
        let mut output = Complex::new(0.0, 0.0);
        for i in 0..self.coefficients.len() {
            output = output + self.coefficients[i] * self.delay_line[i];
        }

        // Compute error
        let error = reference - output;

        // Update coefficients (LMS algorithm)
        for i in 0..self.coefficients.len() {
            let gradient = self.delay_line[i] * error.conj();
            self.coefficients[i] = self.coefficients[i] + gradient * self.step_size;
        }

        output
    }
}

// ============================================================================
// Synchronization
// ============================================================================

/// Carrier synchronization using Costas loop
pub struct CostasLoop {
    /// Loop bandwidth
    bandwidth: f64,

    /// Current phase estimate
    phase: f64,

    /// Current frequency estimate
    freq: f64,

    /// Loop filter state
    integrator: f64,
}

impl CostasLoop {
    /// Create new Costas loop
    pub fn new(bandwidth: f64, _sample_rate: f64) -> Self {
        Self {
            bandwidth,
            phase: 0.0,
            freq: 0.0,
            integrator: 0.0,
        }
    }

    /// Process I/Q samples, return phase-corrected samples
    pub fn process(&mut self, i: f64, q: f64) -> (f64, f64) {
        // Phase detector (QPSK)
        let error = if i.abs() > q.abs() {
            atan2(q * i.signum(), i.abs())
        } else {
            atan2(i * q.signum(), q.abs())
        };

        // Loop filter (proportional + integral)
        self.integrator += error * self.bandwidth;
        let freq_correction = error * self.bandwidth * 2.0 + self.integrator;

        self.freq = freq_correction;
        self.phase += freq_correction;

        // Phase correction
        let phase_cos = cos(self.phase);
        let phase_sin = sin(self.phase);

        let i_corrected = i * phase_cos + q * phase_sin;
        let q_corrected = q * phase_cos - i * phase_sin;

        (i_corrected, q_corrected)
    }

    /// Check if locked
    pub fn is_locked(&self) -> bool {
        self.freq.abs() < 0.1
    }
}

/// Symbol timing synchronizer (Gardner algorithm)
pub struct GardnerTimingLoop {
    /// Current timing offset
    timing_offset: f64,

    /// Loop gain
    gain: f64,

    /// Previous symbol
    prev_symbol: Complex,

    /// Symbol at mid-point
    mid_symbol: Complex,
}

impl GardnerTimingLoop {
    /// Create new timing loop
    pub fn new(gain: f64) -> Self {
        Self {
            timing_offset: 0.0,
            gain,
            prev_symbol: Complex::new(0.0, 0.0),
            mid_symbol: Complex::new(0.0, 0.0),
        }
    }

    /// Process sample, return true if symbol boundary
    pub fn process(&mut self, sample: Complex) -> bool {
        self.timing_offset += self.gain;

        if self.timing_offset >= 1.0 {
            self.timing_offset -= 1.0;

            // Gardner error detector
            let error = (self.mid_symbol.re - self.prev_symbol.re) * sample.re
                + (self.mid_symbol.im - self.prev_symbol.im) * sample.im;

            // Adjust timing
            self.timing_offset += error * 0.1;

            self.prev_symbol = sample;
            true
        } else {
            self.mid_symbol = sample;
            false
        }
    }
}

// ============================================================================
// Modulator/Demodulator
// ============================================================================

/// Soft-decision output
#[derive(Debug, Clone, Copy)]
pub struct SoftDecision {
    /// Log-likelihood ratio (LLR)
    pub llr: f64,

    /// Hard decision
    pub hard: bool,
}

impl SoftDecision {
    /// Create from LLR
    pub fn new(llr: f64) -> Self {
        Self {
            llr,
            hard: llr > 0.0,
        }
    }

    /// Create from channel observation with noise variance
    pub fn from_observation(symbol: f64, noise_var: f64) -> Self {
        let llr = 2.0 * symbol / noise_var;
        Self::new(llr)
    }
}

/// Digital demodulator
pub struct Demodulator {
    /// Modulation scheme
    scheme: ModulationScheme,

    /// Symbol map (constellation points)
    constellation: Vec<Complex>,
}

impl Demodulator {
    /// Create new demodulator
    pub fn new(scheme: ModulationScheme) -> Self {
        let constellation = Self::generate_constellation(scheme);

        Self { scheme, constellation }
    }

    /// Generate constellation points
    fn generate_constellation(scheme: ModulationScheme) -> Vec<Complex> {
        match scheme {
            ModulationScheme::BPSK => vec![Complex::new(-1.0, 0.0), Complex::new(1.0, 0.0)],

            ModulationScheme::QPSK => vec![
                Complex::new(1.0, 1.0),
                Complex::new(-1.0, 1.0),
                Complex::new(-1.0, -1.0),
                Complex::new(1.0, -1.0),
            ],

            ModulationScheme::QAM16 => {
                let levels = [-3.0, -1.0, 1.0, 3.0];
                let mut points = Vec::new();
                for &i in &levels {
                    for &q in &levels {
                        points.push(Complex::new(i / 3.0, q / 3.0));
                    }
                }
                points
            }

            _ => vec![],
        }
    }

    /// Demodulate symbol to bits (hard decision)
    pub fn demodulate_hard(&self, symbol: Complex) -> Vec<bool> {
        // Find nearest constellation point
        let mut nearest_idx = 0;
        let mut min_dist = f64::MAX;

        for (idx, &point) in self.constellation.iter().enumerate() {
            let dist = (symbol - point).mag_sq();
            if dist < min_dist {
                min_dist = dist;
                nearest_idx = idx;
            }
        }

        // Map index to bits
        self.index_to_bits(nearest_idx)
    }

    /// Demodulate symbol to soft decisions
    pub fn demodulate_soft(&self, symbol: Complex, noise_var: f64) -> Vec<SoftDecision> {
        let bits_per_symbol = self.scheme.spectral_efficiency() as usize;
        let mut soft_bits = Vec::with_capacity(bits_per_symbol);

        for bit in 0..bits_per_symbol {
            // Calculate LLR for this bit position
            let mut llr_num = f64::MAX;
            let mut llr_den = f64::MAX;

            for (idx, &point) in self.constellation.iter().enumerate() {
                let bits = self.index_to_bits(idx);
                let dist = (symbol - point).mag_sq();

                if bits[bit] {
                    llr_num = llr_num.min(dist);
                } else {
                    llr_den = llr_den.min(dist);
                }
            }

            let llr = (llr_den - llr_num) / (2.0 * noise_var);
            soft_bits.push(SoftDecision::new(llr));
        }

        soft_bits
    }

    /// Map constellation index to bits
    fn index_to_bits(&self, idx: usize) -> Vec<bool> {
        let n_bits = self.scheme.spectral_efficiency() as usize;
        let mut bits = Vec::with_capacity(n_bits);

        for bit in 0..n_bits {
            bits.push((idx >> (n_bits - 1 - bit)) & 1 != 0);
        }

        bits
    }
}
