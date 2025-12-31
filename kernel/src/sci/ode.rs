//! # Ordinary Differential Equation Solvers
//!
//! Various methods for solving ODEs:
//! - Euler method
//! - Runge-Kutta methods (RK2, RK4)
//! - Adaptive step size methods
//! - Stiff equation solvers
//! - Boundary value problems

use crate::sci::{SciError, SciFloat, SciResult};
use alloc::vec::Vec;

/// ODE system: dy/dt = f(t, y)
///
/// Takes current time t and state y, returns derivatives
pub type ODESystem = dyn Fn(SciFloat, &[SciFloat]) -> Vec<SciFloat>;

/// ODE solution trajectory
#[derive(Debug, Clone)]
pub struct ODESolution {
    /// Time points
    pub t: Vec<SciFloat>,
    /// State at each time point (state[i][j] = j-th component at i-th time)
    pub y: Vec<Vec<SciFloat>>,
}

impl ODESolution {
    /// Create new solution
    pub fn new(t: Vec<SciFloat>, y: Vec<Vec<SciFloat>>) -> Self {
        Self { t, y }
    }

    /// Get solution length
    pub fn len(&self) -> usize {
        self.t.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.t.is_empty()
    }

    /// Interpolate to get state at arbitrary time
    pub fn interpolate(&self, t_query: SciFloat) -> Option<Vec<SciFloat>> {
        if self.t.is_empty() {
            return None;
        }

        // Find surrounding time points
        let idx = match self.t.binary_search_by(|a| a.partial_cmp(&t_query).unwrap()) {
            Ok(i) => return Some(self.y[i].clone()),
            Err(i) if i == 0 => return Some(self.y[0].clone()),
            Err(i) if i >= self.t.len() => return Some(self.y.last()?.clone()),
            Err(i) => i,
        };

        // Linear interpolation
        let t0 = self.t[idx - 1];
        let t1 = self.t[idx];
        let alpha = (t_query - t0) / (t1 - t0);

        let y0 = &self.y[idx - 1];
        let y1 = &self.y[idx];

        let mut result = Vec::with_capacity(y0.len());
        for j in 0..y0.len() {
            result.push(y0[j] + alpha * (y1[j] - y0[j]));
        }

        Some(result)
    }
}

/// Forward Euler method
///
/// Simple first-order method: y_{n+1} = y_n + h * f(t_n, y_n)
pub fn euler(
    f: &ODESystem,
    y0: &[SciFloat],
    t_span: (SciFloat, SciFloat),
    h: SciFloat,
) -> SciResult<ODESolution> {
    let (t0, t_end) = t_span;
    let n = y0.len();

    if h <= 0.0 {
        return Err(SciError::InvalidParameters);
    }

    let num_steps = ((t_end - t0) / h).ceil() as usize;
    let mut t = Vec::with_capacity(num_steps + 1);
    let mut y = Vec::with_capacity(num_steps + 1);

    let mut y_current = y0.to_vec();
    let mut t_current = t0;

    t.push(t_current);
    y.push(y_current.clone());

    while t_current < t_end - h / 2.0 {
        let dydt = f(t_current, &y_current);

        for i in 0..n {
            y_current[i] += h * dydt[i];
        }
        t_current += h;

        t.push(t_current);
        y.push(y_current.clone());
    }

    Ok(ODESolution::new(t, y))
}

/// Runge-Kutta 2nd order (Heun's method)
///
/// y_{n+1} = y_n + h/2 * (k1 + k2)
/// k1 = f(t_n, y_n)
/// k2 = f(t_n + h, y_n + h*k1)
pub fn rk2(
    f: &ODESystem,
    y0: &[SciFloat],
    t_span: (SciFloat, SciFloat),
    h: SciFloat,
) -> SciResult<ODESolution> {
    let (t0, t_end) = t_span;
    let n = y0.len();

    if h <= 0.0 {
        return Err(SciError::InvalidParameters);
    }

    let num_steps = ((t_end - t0) / h).ceil() as usize;
    let mut t = Vec::with_capacity(num_steps + 1);
    let mut y = Vec::with_capacity(num_steps + 1);

    let mut y_current = y0.to_vec();
    let mut t_current = t0;

    t.push(t_current);
    y.push(y_current.clone());

    while t_current < t_end - h / 2.0 {
        // k1
        let k1 = f(t_current, &y_current);

        // k2
        let mut y_temp = Vec::with_capacity(n);
        for i in 0..n {
            y_temp.push(y_current[i] + h * k1[i]);
        }
        let k2 = f(t_current + h, &y_temp);

        // Combine
        for i in 0..n {
            y_current[i] += h * 0.5 * (k1[i] + k2[i]);
        }
        t_current += h;

        t.push(t_current);
        y.push(y_current.clone());
    }

    Ok(ODESolution::new(t, y))
}

/// Runge-Kutta 4th order
///
/// Classic RK4 method with O(h^4) accuracy
pub fn rk4(
    f: &ODESystem,
    y0: &[SciFloat],
    t_span: (SciFloat, SciFloat),
    h: SciFloat,
) -> SciResult<ODESolution> {
    let (t0, t_end) = t_span;
    let n = y0.len();

    if h <= 0.0 {
        return Err(SciError::InvalidParameters);
    }

    let num_steps = ((t_end - t0) / h).ceil() as usize;
    let mut t = Vec::with_capacity(num_steps + 1);
    let mut y = Vec::with_capacity(num_steps + 1);

    let mut y_current = y0.to_vec();
    let mut t_current = t0;

    t.push(t_current);
    y.push(y_current.clone());

    while t_current < t_end - h / 2.0 {
        // k1
        let k1 = f(t_current, &y_current);

        // k2
        let mut y_temp = Vec::with_capacity(n);
        for i in 0..n {
            y_temp.push(y_current[i] + 0.5 * h * k1[i]);
        }
        let k2 = f(t_current + 0.5 * h, &y_temp);

        // k3
        for i in 0..n {
            y_temp[i] = y_current[i] + 0.5 * h * k2[i];
        }
        let k3 = f(t_current + 0.5 * h, &y_temp);

        // k4
        for i in 0..n {
            y_temp[i] = y_current[i] + h * k3[i];
        }
        let k4 = f(t_current + h, &y_temp);

        // Combine
        for i in 0..n {
            y_current[i] += (h / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        t_current += h;

        t.push(t_current);
        y.push(y_current.clone());
    }

    Ok(ODESolution::new(t, y))
}

/// Adaptive step size Runge-Kutta (Cash-Karp method)
///
/// Uses embedded Runge-Kutta formula for error estimation
pub struct AdaptiveRK45 {
    /// Initial step size
    pub h_init: SciFloat,
    /// Minimum step size
    pub h_min: SciFloat,
    /// Maximum step size
    pub h_max: SciFloat,
    /// Error tolerance
    pub tolerance: SciFloat,
    /// Safety factor for step adjustment
    pub safety_factor: SciFloat,
}

impl Default for AdaptiveRK45 {
    fn default() -> Self {
        Self {
            h_init: 0.1,
            h_min: 1e-10,
            h_max: 1.0,
            tolerance: 1e-6,
            safety_factor: 0.9,
        }
    }
}

impl AdaptiveRK45 {
    /// Solve with adaptive step size
    pub fn solve(
        &self,
        f: &ODESystem,
        y0: &[SciFloat],
        t_span: (SciFloat, SciFloat),
    ) -> SciResult<ODESolution> {
        let (t0, t_end) = t_span;
        let n = y0.len();

        let mut t = Vec::new();
        let mut y = Vec::new();

        let mut y_current = y0.to_vec();
        let mut t_current = t0;
        let mut h = self.h_init;

        t.push(t_current);
        y.push(y_current.clone());

        while t_current < t_end {
            // Limit step to not exceed t_end
            if t_current + h > t_end {
                h = t_end - t_current;
            }

            // Cash-Karp coefficients
            let k1 = f(t_current, &y_current);

            let mut y_temp = vec![0.0; n];
            for i in 0..n {
                y_temp[i] = y_current[i] + h * k1[i] / 5.0;
            }
            let k2 = f(t_current + h / 5.0, &y_temp);

            for i in 0..n {
                y_temp[i] = y_current[i] + h * (3.0 * k1[i] + 9.0 * k2[i]) / 40.0;
            }
            let k3 = f(t_current + 3.0 * h / 10.0, &y_temp);

            for i in 0..n {
                y_temp[i] = y_current[i] + h * (3.0 * k1[i] - 9.0 * k2[i] + 12.0 * k3[i]) / 10.0;
            }
            let k4 = f(t_current + 3.0 * h / 5.0, &y_temp);

            for i in 0..n {
                y_temp[i] =
                    y_current[i] + h * (-11.0 * k1[i] + 54.0 * k2[i] - 50.0 * k3[i] + 40.0 * k4[i])
                        / 120.0;
            }
            let k5 = f(t_current + h, &y_temp);

            for i in 0..n {
                y_temp[i] = y_current[i]
                    + h * (-23.0 * k1[i] + 237.0 * k2[i] - 580.0 * k3[i] + 480.0 * k4[i]
                        - 135.0 * k5[i])
                        / 480.0;
            }
            let k6 = f(t_current + 7.0 * h / 8.0, &y_temp);

            // 5th order solution
            let mut y5 = vec![0.0; n];
            for i in 0..n {
                y5[i] = y_current[i]
                    + h * (37.0 * k1[i] + 250.0 * k3[i] + 125.0 * k4[i] + 512.0 * k6[i])
                        / 1368.0;
            }

            // 4th order solution
            let mut y4 = vec![0.0; n];
            for i in 0..n {
                y4[i] = y_current[i]
                    + h
                        * (2825.0 * k1[i] + 8963.0 * k3[i] - 3592.0 * k4[i] - 2889.0 * k5[i]
                            + 1929.0 * k6[i])
                        / 14160.0;
            }

            // Error estimate
            let mut error = 0.0;
            for i in 0..n {
                let diff = (y5[i] - y4[i]).abs();
                let scale = 1.0 + y5[i].abs();
                error += (diff / scale).powi(2);
            }
            error = (error / n as SciFloat).sqrt();

            // Step size adjustment
            if error < self.tolerance {
                // Accept step
                t_current += h;
                y_current = y5;

                t.push(t_current);
                y.push(y_current.clone());

                // Increase step size
                h = (h * self.safety_factor * (self.tolerance / error).powf(0.2)).min(self.h_max);
            } else {
                // Reject step, decrease step size
                h = (h * self.safety_factor * (self.tolerance / error).powf(0.25))
                    .max(self.h_min);
            }

            if h < self.h_min {
                return Err(SciError::NotConverged);
            }
        }

        Ok(ODESolution::new(t, y))
    }
}

/// Backward Euler method for stiff equations
///
/// Implicit method: y_{n+1} = y_n + h * f(t_{n+1}, y_{n+1})
/// Uses fixed-point iteration to solve implicit equation
pub fn backward_euler(
    f: &ODESystem,
    y0: &[SciFloat],
    t_span: (SciFloat, SciFloat),
    h: SciFloat,
    max_iter: usize,
    tol: SciFloat,
) -> SciResult<ODESolution> {
    let (t0, t_end) = t_span;
    let n = y0.len();

    if h <= 0.0 {
        return Err(SciError::InvalidParameters);
    }

    let num_steps = ((t_end - t0) / h).ceil() as usize;
    let mut t = Vec::with_capacity(num_steps + 1);
    let mut y = Vec::with_capacity(num_steps + 1);

    let mut y_current = y0.to_vec();
    let mut t_current = t0;

    t.push(t_current);
    y.push(y_current.clone());

    while t_current < t_end - h / 2.0 {
        // Fixed-point iteration
        let mut y_next = y_current.clone();
        let mut y_old;

        for _iter in 0..max_iter {
            y_old = y_next.clone();
            let dydt = f(t_current + h, &y_next);

            for i in 0..n {
                y_next[i] = y_current[i] + h * dydt[i];
            }

            // Check convergence
            let mut diff = 0.0;
            for i in 0..n {
                diff += (y_next[i] - y_old[i]).powi(2);
            }
            diff = diff.sqrt();

            if diff < tol {
                break;
            }
        }

        y_current = y_next;
        t_current += h;

        t.push(t_current);
        y.push(y_current.clone());
    }

    Ok(ODESolution::new(t, y))
}

/// Shooting method for boundary value problems
///
/// Converts BVP to IVP and uses root-finding
pub fn shooting_method(
    f: &ODESystem,
    bc_left: &[SciFloat],
    bc_right: &[SciFloat],
    t_span: (SciFloat, SciFloat),
    h: SciFloat,
    tol: SciFloat,
) -> SciResult<ODESolution> {
    let n = bc_left.len();
    let m = bc_right.len();

    if n + m != f(0.0, &[0.0; 10]).len() {
        return Err(SciError::InvalidDimensions);
    }

    // Simple bisection for scalar unknown (simplified case)
    // In general, would use multi-dimensional root finding

    // For simplicity, assume we have one unknown initial condition
    let mut low = -10.0;
    let mut high = 10.0;

    for _iter in 0..100 {
        let mid = 0.5 * (low + high);

        // Initial condition guess
        let mut y0 = vec![0.0; n];
        y0[0] = mid;

        // Solve IVP
        let sol = rk4(f, &y0, t_span, h)?;

        // Check boundary condition at right
        let y_final = &sol.y[sol.y.len() - 1];
        let residual = bc_right[0] - y_final[0];

        if residual.abs() < tol {
            return Ok(sol);
        }

        if residual > 0.0 {
            low = mid;
        } else {
            high = mid;
        }
    }

    Err(SciError::NotConverged)
}

/// Finite difference method for boundary value problems
///
/// Discretize domain and solve linear system
pub fn finite_difference_bvp(
    f: &dyn Fn(SciFloat, SciFloat, SciFloat) -> SciFloat, // y'' = f(t, y, y')
    bc: (SciFloat, SciFloat),                              // Boundary values (y[0], y[1])
    t_span: (SciFloat, SciFloat),
    n: usize,
) -> SciResult<ODESolution> {
    let (t0, t_end) = t_span;
    let h = (t_end - t0) / (n as SciFloat);

    if n < 2 {
        return Err(SciError::InvalidDimensions);
    }

    // Build tridiagonal system
    let mut a = vec![0.0; n - 1]; // Lower diagonal
    let mut b = vec![0.0; n - 1]; // Main diagonal
    let mut c = vec![0.0; n - 1]; // Upper diagonal
    let mut d = vec![0.0; n - 1]; // Right-hand side

    for i in 0..n - 1 {
        let t = t0 + (i + 1) as SciFloat * h;

        a[i] = 1.0 / h.powi(2);
        b[i] = -2.0 / h.powi(2);
        c[i] = 1.0 / h.powi(2);

        // RHS including boundary conditions
        let rhs = f(t, 0.0, 0.0); // Simplified, assuming linear
        d[i] = rhs;

        if i == 0 {
            d[0] -= bc.0 * a[0];
        }
        if i == n - 2 {
            d[n - 2] -= bc.1 * c[n - 2];
        }
    }

    // Solve tridiagonal system (Thomas algorithm)
    let mut y = vec![0.0; n + 1];
    y[0] = bc.0;
    y[n] = bc.1;

    // Forward elimination
    for i in 1..n - 1 {
        let factor = a[i] / b[i - 1];
        b[i] -= factor * c[i - 1];
        d[i] -= factor * d[i - 1];
    }

    // Back substitution
    y[n - 1] = d[n - 2] / b[n - 2];
    for i in (1..n - 1).rev() {
        y[i] = (d[i - 1] - c[i - 1] * y[i + 1]) / b[i - 1];
    }

    // Create solution
    let t: Vec<SciFloat> = (0..=n).map(|i| t0 + i as SciFloat * h).collect();
    let y_vec: Vec<Vec<SciFloat>> = y.iter().map(|&yi| vec![yi]).collect();

    Ok(ODESolution::new(t, y_vec))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test equation: dy/dt = -y, y(0) = 1
    // Solution: y(t) = exp(-t)
    fn exponential_decay(_t: SciFloat, y: &[SciFloat]) -> Vec<SciFloat> {
        vec![-y[0]]
    }

    // Harmonic oscillator: d^2y/dt^2 = -y
    // State: [y, dy/dt]
    fn harmonic_oscillator(_t: SciFloat, y: &[SciFloat]) -> Vec<SciFloat> {
        vec![y[1], -y[0]]
    }

    #[test]
    fn test_euler() {
        let sol = euler(&exponential_decay, &[1.0], (0.0, 1.0), 0.1).unwrap();

        // Check final value
        let y_final = sol.y.last().unwrap()[0];
        let expected = (-sol.t.last().unwrap()).exp();

        assert!((y_final - expected).abs() < 0.01); // Euler is first order
    }

    #[test]
    fn test_rk4() {
        let sol = rk4(&exponential_decay, &[1.0], (0.0, 1.0), 0.1).unwrap();

        let y_final = sol.y.last().unwrap()[0];
        let expected = (-sol.t.last().unwrap()).exp();

        assert!((y_final - expected).abs() < 1e-6); // RK4 is fourth order
    }

    #[test]
    fn test_rk2() {
        let sol = rk2(&exponential_decay, &[1.0], (0.0, 1.0), 0.1).unwrap();

        let y_final = sol.y.last().unwrap()[0];
        let expected = (-sol.t.last().unwrap()).exp();

        assert!((y_final - expected).abs() < 0.01); // RK2 is second order
    }

    #[test]
    fn test_adaptive_rk45() {
        let solver = AdaptiveRK45::default();
        let sol = solver
            .solve(&exponential_decay, &[1.0], (0.0, 1.0))
            .unwrap();

        let y_final = sol.y.last().unwrap()[0];
        let expected = (-sol.t.last().unwrap()).exp();

        assert!((y_final - expected).abs() < 1e-5);
    }

    #[test]
    fn test_harmonic_oscillator() {
        let sol = rk4(&harmonic_oscillator, &[1.0, 0.0], (0.0, 2.0 * core::f64::consts::PI), 0.01)
            .unwrap();

        // After one full period, should return to initial state
        let y_final = &sol.y.last().unwrap();

        assert!((y_final[0] - 1.0).abs() < 0.01);
        assert!((y_final[1] - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_backward_euler() {
        // Stiff equation: dy/dt = -100y, y(0) = 1
        let stiff = |_t: SciFloat, y: &[SciFloat]| -> Vec<SciFloat> { vec![-100.0 * y[0]] };

        let sol = backward_euler(&stiff, &[1.0], (0.0, 0.1), 0.01, 100, 1e-6).unwrap();

        // Should not blow up
        let y_final = sol.y.last().unwrap()[0];
        assert!(y_final > 0.0);
        assert!(y_final < 1.0);
    }

    #[test]
    fn test_interpolation() {
        let sol = rk4(&exponential_decay, &[1.0], (0.0, 1.0), 0.1).unwrap();

        // Interpolate at t = 0.5
        let y_interp = sol.interpolate(0.5).unwrap();
        let expected = (-0.5).exp();

        assert!((y_interp[0] - expected).abs() < 0.01);
    }
}
