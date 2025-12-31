//! # FFT and Signal Processing
//!
//! Fast Fourier Transform and signal processing operations:
//! - Cooley-Tukey FFT algorithm
//! - Inverse FFT
//! - Convolution and correlation
//! - Power spectral density
//! - Window functions

use crate::sci::{SciComplex, SciFloat, SciResult};
use alloc::vec::Vec;
use core::f64::consts::PI;

/// FFT size must be a power of 2 for Cooley-Tukey algorithm
pub fn is_power_of_two(n: usize) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

/// Compute next power of 2
pub fn next_power_of_two(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut power = 1;
    while power < n {
        power *= 2;
    }
    power
}

/// Cooley-Tukey FFT algorithm (in-place, decimation-in-time)
///
/// # Arguments
/// * `data` - Input/output complex data (must have length as power of 2)
/// * `inverse` - If true, compute inverse FFT
pub fn fft(data: &mut [SciComplex], inverse: bool) -> SciResult<()> {
    let n = data.len();

    if !is_power_of_two(n) {
        return Err(crate::sci::SciError::InvalidParameters);
    }

    // Bit-reversal permutation
    bit_reverse(data);

    // Cooley-Tukey FFT
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let angle = if inverse {
            2.0 * PI / len as SciFloat
        } else {
            -2.0 * PI / len as SciFloat
        };
        let wlen = SciComplex::new(angle.cos(), angle.sin());

        for i in (0..n).step_by(len) {
            let mut w = SciComplex::new(1.0, 0.0);
            for j in 0..half {
                let u = data[i + j];
                let v = data[i + j + half] * w;
                data[i + j] = u + v;
                data[i + j + half] = u - v;
                w *= wlen;
            }
        }
        len *= 2;
    }

    // Normalize for inverse FFT
    if inverse {
        let scale = 1.0 / n as SciFloat;
        for x in data.iter_mut() {
            *x *= scale;
        }
    }

    Ok(())
}

/// Bit-reversal permutation for FFT
fn bit_reverse(data: &mut [SciComplex]) {
    let n = data.len();
    let mut j = 0;

    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;

        if i < j {
            data.swap(i, j);
        }
    }
}

/// Compute FFT of real-valued input (optimized)
///
/// Uses the fact that for real input, X[k] = conj(X[N-k])
pub fn fft_real(input: &[SciFloat]) -> SciResult<Vec<SciComplex>> {
    let n = input.len();

    if !is_power_of_two(n) {
        return Err(crate::sci::SciError::InvalidParameters);
    }

    // Pack real data into complex format
    let mut data: Vec<SciComplex> = input.iter().map(|&x| SciComplex::new(x, 0.0)).collect();

    fft(&mut data, false)?;
    Ok(data)
}

/// Compute inverse FFT to get real-valued output
pub fn ifft_real(data: &[SciComplex]) -> SciResult<Vec<SciFloat>> {
    let mut data = data.to_vec();
    fft(&mut data, true)?;

    // Extract real parts (imaginary should be near zero)
    Ok(data.iter().map(|x| x.re).collect())
}

/// Convolution using FFT (overlap-add method for arbitrary sizes)
///
/// For linear convolution of two sequences
pub fn convolve(x: &[SciFloat], h: &[SciFloat]) -> SciResult<Vec<SciFloat>> {
    let n = x.len();
    let m = h.len();
    let l = n + m - 1;

    // Pad to power of 2 for efficiency
    let size = next_power_of_two(l);

    // Zero-pad both sequences
    let mut x_padded = vec![SciComplex::new(0.0, 0.0); size];
    let mut h_padded = vec![SciComplex::new(0.0, 0.0); size];

    for (i, &xi) in x.iter().enumerate() {
        x_padded[i] = SciComplex::new(xi, 0.0);
    }
    for (i, &hi) in h.iter().enumerate() {
        h_padded[i] = SciComplex::new(hi, 0.0);
    }

    // FFT
    fft(&mut x_padded, false)?;
    fft(&mut h_padded, false)?;

    // Point-wise multiplication
    for i in 0..size {
        x_padded[i] *= h_padded[i];
    }

    // IFFT
    fft(&mut x_padded, true)?;

    // Extract result
    let mut result = vec![0.0; l];
    for i in 0..l {
        result[i] = x_padded[i].re;
    }

    Ok(result)
}

/// Cross-correlation using FFT
///
/// Computes correlation between two sequences
pub fn correlate(x: &[SciFloat], y: &[SciFloat]) -> SciResult<Vec<SciFloat>> {
    // Cross-correlation = convolution with time-reversed signal
    let y_reversed: Vec<SciFloat> = y.iter().rev().cloned().collect();
    convolve(x, &y_reversed)
}

/// Auto-correlation using FFT
pub fn autocorrelate(x: &[SciFloat]) -> SciResult<Vec<SciFloat>> {
    correlate(x, x)
}

/// Power spectral density (PSD) using Welch's method
///
/// # Arguments
/// * `x` - Input signal
/// * `sample_rate` - Sampling rate in Hz
/// * `nfft` - FFT size (must be power of 2)
/// * `overlap` - Number of samples to overlap between segments
pub fn power_spectral_density(
    x: &[SciFloat],
    sample_rate: SciFloat,
    nfft: usize,
    overlap: usize,
) -> SciResult<(Vec<SciFloat>, Vec<SciFloat>)> {
    if !is_power_of_two(nfft) {
        return Err(crate::sci::SciError::InvalidParameters);
    }

    let segment_len = nfft;
    let step = segment_len - overlap;

    if x.len() < segment_len {
        return Err(crate::sci::SciError::InvalidDimensions);
    }

    let num_segments = (x.len() - overlap) / step;
    let mut psd_sum = vec![0.0; nfft / 2 + 1];

    for i in 0..num_segments {
        let start = i * step;
        let end = start + segment_len;
        let segment = &x[start..end];

        // Apply window (Hanning by default)
        let window = hanning_window(segment_len);
        let windowed: Vec<SciFloat> = segment
            .iter()
            .zip(window.iter())
            .map(|(&x, &w)| x * w)
            .collect();

        // FFT
        let mut fft_data: Vec<SciComplex> =
            windowed.iter().map(|&x| SciComplex::new(x, 0.0)).collect();

        fft(&mut fft_data, false)?;

        // Compute power (magnitude squared)
        for j in 0..=nfft / 2 {
            let mag_sq = fft_data[j].norm_sqr();
            psd_sum[j] += mag_sq;
        }
    }

    // Average
    let num_seg_float = num_segments as SciFloat;
    for psd in psd_sum.iter_mut() {
        *psd /= num_seg_float;
    }

    // Normalize for power
    let scale = 1.0 / (sample_rate * segment_len as SciFloat);
    for psd in psd_sum.iter_mut() {
        *psd *= scale;
    }

    // Frequency axis
    let freqs: Vec<SciFloat> = (0..=nfft / 2)
        .map(|i| i as SciFloat * sample_rate / nfft as SciFloat)
        .collect();

    Ok((freqs, psd_sum))
}

/// Window functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    Rectangular,
    Hanning,
    Hamming,
    Blackman,
    FlatTop,
}

/// Generate window function
pub fn window(window_type: WindowType, n: usize) -> Vec<SciFloat> {
    match window_type {
        WindowType::Rectangular => vec![1.0; n],
        WindowType::Hanning => hanning_window(n),
        WindowType::Hamming => hamming_window(n),
        WindowType::Blackman => blackman_window(n),
        WindowType::FlatTop => flattop_window(n),
    }
}

/// Hanning window
pub fn hanning_window(n: usize) -> Vec<SciFloat> {
    if n < 2 {
        return vec![1.0];
    }
    let n_float = n as SciFloat;
    (0..n)
        .map(|i| {
            let angle = 2.0 * PI * i as SciFloat / (n_float - 1.0);
            0.5 - 0.5 * angle.cos()
        })
        .collect()
}

/// Hamming window
pub fn hamming_window(n: usize) -> Vec<SciFloat> {
    if n < 2 {
        return vec![1.0];
    }
    let n_float = n as SciFloat;
    (0..n)
        .map(|i| {
            let angle = 2.0 * PI * i as SciFloat / (n_float - 1.0);
            0.54 - 0.46 * angle.cos()
        })
        .collect()
}

/// Blackman window
pub fn blackman_window(n: usize) -> Vec<SciFloat> {
    if n < 2 {
        return vec![1.0];
    }
    let n_float = n as SciFloat;
    (0..n)
        .map(|i| {
            let angle1 = 2.0 * PI * i as SciFloat / (n_float - 1.0);
            let angle2 = 4.0 * PI * i as SciFloat / (n_float - 1.0);
            0.42 - 0.5 * angle1.cos() + 0.08 * angle2.cos()
        })
        .collect()
}

/// Flat top window
pub fn flattop_window(n: usize) -> Vec<SciFloat> {
    if n < 2 {
        return vec![1.0];
    }
    let n_float = n as SciFloat;
    (0..n)
        .map(|i| {
            let angle1 = 2.0 * PI * i as SciFloat / (n_float - 1.0);
            let angle2 = 4.0 * PI * i as SciFloat / (n_float - 1.0);
            let angle3 = 6.0 * PI * i as SciFloat / (n_float - 1.0);
            let angle4 = 8.0 * PI * i as SciFloat / (n_float - 1.0);
            0.21557895 - 0.41663158 * angle1.cos() + 0.277263158 * angle2.cos()
                - 0.083578947 * angle3.cos()
                + 0.006947368 * angle4.cos()
        })
        .collect()
}

/// Spectrogram computation
///
/// # Arguments
/// * `x` - Input signal
/// * `sample_rate` - Sampling rate
/// * `window_size` - Size of each window
/// * `overlap` - Overlap between windows
pub fn spectrogram(
    x: &[SciFloat],
    sample_rate: SciFloat,
    window_size: usize,
    overlap: usize,
) -> SciResult<(Vec<Vec<SciFloat>>, Vec<SciFloat>, Vec<SciFloat>)> {
    let step = window_size - overlap;

    if x.len() < window_size || step == 0 {
        return Err(crate::sci::SciError::InvalidDimensions);
    }

    let num_frames = (x.len() - overlap) / step;
    let nfft = next_power_of_two(window_size);

    // Frequency axis
    let freqs: Vec<SciFloat> = (0..=nfft / 2)
        .map(|i| i as SciFloat * sample_rate / nfft as SciFloat)
        .collect();

    // Time axis
    let times: Vec<SciFloat> = (0..num_frames)
        .map(|i| (i * step) as SciFloat / sample_rate)
        .collect();

    // Compute FFT for each frame
    let mut spec = vec![vec![0.0; nfft / 2 + 1]; num_frames];

    for frame_idx in 0..num_frames {
        let start = frame_idx * step;
        let end = start + window_size;

        let mut segment = vec![SciComplex::new(0.0, 0.0); nfft];
        let win = hanning_window(window_size);

        for (i, &xi) in x[start..end].iter().enumerate() {
            segment[i] = SciComplex::new(xi * win[i], 0.0);
        }

        fft(&mut segment, false)?;

        for i in 0..=nfft / 2 {
            // Magnitude in dB
            let mag = segment[i].norm();
            spec[frame_idx][i] = 20.0 * (mag + 1e-10).log10();
        }
    }

    Ok((spec, times, freqs))
}

/// Short-time Fourier transform (STFT)
pub fn stft(
    x: &[SciFloat],
    window_size: usize,
    hop_size: usize,
) -> SciResult<Vec<Vec<SciComplex>>> {
    if x.len() < window_size {
        return Err(crate::sci::SciError::InvalidDimensions);
    }

    let nfft = next_power_of_two(window_size);
    let num_frames = (x.len() - window_size) / hop_size + 1;
    let win = hanning_window(window_size);

    let mut result = Vec::with_capacity(num_frames);

    for frame_idx in 0..num_frames {
        let start = frame_idx * hop_size;
        let end = start + window_size;

        let mut segment = vec![SciComplex::new(0.0, 0.0); nfft];

        for (i, &xi) in x[start..end].iter().enumerate() {
            segment[i] = SciComplex::new(xi * win[i], 0.0);
        }

        fft(&mut segment, false)?;

        // Keep only positive frequencies
        result.push(segment[..=nfft / 2].to_vec());
    }

    Ok(result)
}

/// Inverse STFT
pub fn istft(
    stft_data: &[Vec<SciComplex>],
    window_size: usize,
    hop_size: usize,
) -> SciResult<Vec<SciFloat>> {
    if stft_data.is_empty() {
        return Ok(Vec::new());
    }

    let nfft = stft_data[0].len() * 2 - 1;
    let num_frames = stft_data.len();
    let signal_len = (num_frames - 1) * hop_size + window_size;

    let mut result = vec![0.0; signal_len];
    let mut window_sum = vec![0.0; signal_len];
    let win = hanning_window(window_size);

    for (frame_idx, frame) in stft_data.iter().enumerate() {
        let start = frame_idx * hop_size;

        // Reconstruct full spectrum (conjugate symmetry)
        let mut full_spectrum = vec![SciComplex::new(0.0, 0.0); nfft];
        for (i, &val) in frame.iter().enumerate() {
            full_spectrum[i] = val;
            if i > 0 && i < frame.len() - 1 {
                full_spectrum[nfft - i] = val.conj();
            }
        }

        // IFFT
        fft(&mut full_spectrum, true)?;

        // Overlap-add
        for i in 0..window_size {
            result[start + i] += full_spectrum[i].re * win[i];
            window_sum[start + i] += win[i] * win[i];
        }
    }

    // Normalize
    for i in 0..result.len() {
        if window_sum[i] > 1e-10 {
            result[i] /= window_sum[i];
        }
    }

    Ok(result)
}

/// Goertzel algorithm - efficient single-bin DFT
///
/// Useful for detecting specific frequencies (e.g., DTMF tones)
pub fn goertzel(x: &[SciFloat], target_freq: SciFloat, sample_rate: SciFloat) -> SciComplex {
    let n = x.len();
    let k = (target_freq * n as SciFloat / sample_rate) as usize;
    let omega = 2.0 * PI * k as SciFloat / n as SciFloat;
    let coeff = 2.0 * omega.cos();
    let sine = omega.sin();
    let cosine = omega.cos();

    let mut s0 = 0.0;
    let mut s1 = 0.0;
    let mut s2 = 0.0;

    for &xi in x.iter() {
        s0 = coeff * s1 - s2 + xi;
        s2 = s1;
        s1 = s0;
    }

    let real = s1 - s2 * cosine;
    let imag = s2 * sine;

    SciComplex::new(real, imag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_of_two() {
        assert!(is_power_of_two(1));
        assert!(is_power_of_two(2));
        assert!(is_power_of_two(4));
        assert!(is_power_of_two(1024));
        assert!(!is_power_of_two(3));
        assert!(!is_power_of_two(100));
    }

    #[test]
    fn test_next_power_of_two() {
        assert_eq!(next_power_of_two(1), 1);
        assert_eq!(next_power_of_two(5), 8);
        assert_eq!(next_power_of_two(100), 128);
        assert_eq!(next_power_of_two(1024), 1024);
    }

    #[test]
    fn test_fft_round_trip() {
        let n = 8;
        let mut data: Vec<SciComplex> = (0..n)
            .map(|i| SciComplex::new(i as SciFloat, 0.0))
            .collect();

        let original = data.clone();

        fft(&mut data, false).unwrap();
        fft(&mut data, true).unwrap();

        for i in 0..n {
            assert!((data[i].re - original[i].re).abs() < 1e-10);
            assert!((data[i].im - original[i].im).abs() < 1e-10);
        }
    }

    #[test]
    fn test_fft_real() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let result = fft_real(&x).unwrap();

        // DC component should be sum of input
        assert!((result[0].re - 36.0).abs() < 1e-10);

        // For real input, we should have conjugate symmetry
        for i in 1..x.len() / 2 {
            let diff = result[i] - result[x.len() - i].conj();
            assert!(diff.norm() < 1e-10);
        }
    }

    #[test]
    fn test_convolution() {
        let x = vec![1.0, 2.0, 3.0];
        let h = vec![1.0, 1.0];

        let result = convolve(&x, &h).unwrap();

        // Expected: [1*1, 1*2+2*1, 1*3+2*2, 2*3, 3*1] = [1, 4, 7, 6, 3]
        // Wait, that's not right. Let me recalculate.
        // Linear convolution: y[n] = sum(x[k] * h[n-k])
        // y[0] = x[0]*h[0] = 1*1 = 1
        // y[1] = x[0]*h[1] + x[1]*h[0] = 1*1 + 2*1 = 3
        // y[2] = x[1]*h[1] + x[2]*h[0] = 2*1 + 3*1 = 5
        // y[3] = x[2]*h[1] = 3*1 = 3

        assert_eq!(result.len(), 4);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 3.0).abs() < 1e-10);
        assert!((result[2] - 5.0).abs() < 1e-10);
        assert!((result[3] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_autocorrelation() {
        let x = vec![1.0, 2.0, 3.0, 2.0, 1.0];
        let result = autocorrelate(&x).unwrap();

        // Peak should be at zero lag
        let max_idx = result
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;

        // Center of autocorrelation
        let center = x.len() - 1;
        assert_eq!(max_idx, center);
    }

    #[test]
    fn test_window_functions() {
        let n = 5;

        // Rectangular window
        let rect = window(WindowType::Rectangular, n);
        assert_eq!(rect.len(), n);
        assert!(rect.iter().all(|&x| (x - 1.0).abs() < 1e-10));

        // Hanning window
        let hann = hanning_window(n);
        assert_eq!(hann.len(), n);
        assert!(hann.iter().all(|&x| x >= 0.0 && x <= 1.0));

        // Windows should be symmetric
        let hamming = hamming_window(n);
        for i in 0..n / 2 {
            assert!((hamming[i] - hamming[n - 1 - i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_spectrogram() {
        let sample_rate = 1000.0;
        let n = 1000;

        // Create a simple test signal: 10 Hz + 50 Hz sine waves
        let mut x = vec![0.0; n];
        for i in 0..n {
            let t = i as SciFloat / sample_rate;
            x[i] = (2.0 * PI * 10.0 * t).sin() + 0.5 * (2.0 * PI * 50.0 * t).sin();
        }

        let (spec, times, freqs) =
            spectrogram(&x, sample_rate, 256, 128).unwrap();

        assert!(!spec.is_empty());
        assert!(!times.is_empty());
        assert!(!freqs.is_empty());
        assert_eq!(spec.len(), times.len());
        assert_eq!(spec[0].len(), freqs.len());
    }

    #[test]
    fn test_stft_round_trip() {
        let sample_rate = 1000.0;
        let n = 512;

        // Create test signal
        let mut x = vec![0.0; n];
        for i in 0..n {
            let t = i as SciFloat / sample_rate;
            x[i] = (2.0 * PI * 10.0 * t).sin();
        }

        // STFT
        let stft_data = stft(&x, 64, 32).unwrap();

        // ISTFT
        let x_reconstructed = istft(&stft_data, 64, 32).unwrap();

        // Check reconstruction (accounting for windowing effects)
        assert_eq!(x_reconstructed.len(), x.len());

        // Compute relative error
        let mut error = 0.0;
        for i in 0..x.len().min(x_reconstructed.len()) {
            error += (x[i] - x_reconstructed[i]).abs();
        }
        error /= x.len() as SciFloat;

        // Should be reasonably accurate
        assert!(error < 0.5);
    }

    #[test]
    fn test_goertzel() {
        let sample_rate = 1000.0;
        let n = 100;

        // Create 10 Hz sine wave
        let mut x = vec![0.0; n];
        for i in 0..n {
            let t = i as SciFloat / sample_rate;
            x[i] = (2.0 * PI * 10.0 * t).sin();
        }

        // Detect 10 Hz component
        let result = goertzel(&x, 10.0, sample_rate);

        // Should have significant magnitude
        assert!(result.norm() > n as SciFloat / 2.0);

        // Detect different frequency (should be smaller)
        let result_wrong = goertzel(&x, 25.0, sample_rate);
        assert!(result_wrong.norm() < result.norm());
    }
}
