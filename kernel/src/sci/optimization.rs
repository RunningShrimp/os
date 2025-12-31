//! # Numerical Optimization Algorithms
//!
//! Provides various optimization methods for finding minima/maxima of functions:
//! - Gradient descent
//! - Newton and quasi-Newton methods (BFGS)
//! - - Conjugate gradient
//! - Constrained optimization
//! - Global optimization (simulated annealing, genetic algorithms)

use crate::sci::{SciError, SciFloat, SciResult};
use alloc::vec::Vec;

/// Gradient of a multivariate function
pub type Gradient = Vec<SciFloat>;

/// Hessian matrix of a multivariate function
pub type Hessian = Vec<Vec<SciFloat>>;

/// Multivariate objective function: f(x) -> scalar
pub type ObjectiveFunction = dyn Fn(&[SciFloat]) -> SciFloat;

/// Gradient of objective function: ∇f(x)
pub type GradientFunction = dyn Fn(&[SciFloat]) -> Gradient;

/// Optimization algorithm configuration
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    /// Maximum number of iterations
    pub max_iterations: usize,
    /// Tolerance for convergence
    pub tolerance: SciFloat,
    /// Learning rate (for gradient-based methods)
    pub learning_rate: SciFloat,
    /// Minimum step size
    pub min_step: SciFloat,
    /// Verbose output
    pub verbose: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            tolerance: 1e-6,
            learning_rate: 0.01,
            min_step: 1e-10,
            verbose: false,
        }
    }
}

/// Optimization result
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Optimal point
    pub x: Vec<SciFloat>,
    /// Optimal function value
    pub value: SciFloat,
    /// Number of iterations performed
    pub iterations: usize,
    /// Whether convergence was achieved
    pub converged: bool,
    /// Final gradient norm
    pub gradient_norm: SciFloat,
}

/// Gradient descent optimizer
pub struct GradientDescent {
    config: OptimizationConfig,
    momentum: Option<SciFloat>,
}

impl GradientDescent {
    /// Create a new gradient descent optimizer
    pub fn new(config: OptimizationConfig) -> Self {
        Self {
            config,
            momentum: None,
        }
    }

    /// Enable momentum with given coefficient
    pub fn with_momentum(mut self, coefficient: SciFloat) -> Self {
        self.momentum = Some(coefficient);
        self
    }

    /// Minimize function using gradient descent
    pub fn minimize(
        &self,
        f: &ObjectiveFunction,
        grad: &GradientFunction,
        x0: &[SciFloat],
    ) -> SciResult<OptimizationResult> {
        let n = x0.len();
        let mut x = x0.to_vec();
        let mut velocity = vec![0.0; n];
        let mut prev_value = f(&x);

        for iter in 0..self.config.max_iterations {
            let g = grad(&x);
            let grad_norm: SciFloat = g.iter().map(|&gi| gi * gi).sum::<SciFloat>().sqrt();

            if grad_norm < self.config.tolerance {
                return Ok(OptimizationResult {
                    x,
                    value: prev_value,
                    iterations: iter,
                    converged: true,
                    gradient_norm: grad_norm,
                });
            }

            // Update with momentum if enabled
            if let Some(alpha) = self.momentum {
                for i in 0..n {
                    velocity[i] = alpha * velocity[i] + self.config.learning_rate * g[i];
                    x[i] -= velocity[i];
                }
            } else {
                for i in 0..n {
                    x[i] -= self.config.learning_rate * g[i];
                }
            }

            let value = f(&x);

            // Check for convergence
            if (value - prev_value).abs() < self.config.tolerance {
                return Ok(OptimizationResult {
                    x,
                    value,
                    iterations: iter + 1,
                    converged: true,
                    gradient_norm: grad_norm,
                });
            }

            prev_value = value;
        }

        // Did not converge
        let g = grad(&x);
        let grad_norm: SciFloat = g.iter().map(|&gi| gi * gi).sum::<SciFloat>().sqrt();

        Ok(OptimizationResult {
            x,
            value: prev_value,
            iterations: self.config.max_iterations,
            converged: false,
            gradient_norm: grad_norm,
        })
    }
}

/// Newton's method optimizer
pub struct NewtonMethod {
    config: OptimizationConfig,
}

impl NewtonMethod {
    /// Create a new Newton method optimizer
    pub fn new(config: OptimizationConfig) -> Self {
        Self { config }
    }

    /// Minimize function using Newton's method
    pub fn minimize(
        &self,
        f: &ObjectiveFunction,
        grad: &GradientFunction,
        hessian: &dyn Fn(&[SciFloat]) -> Hessian,
        x0: &[SciFloat],
    ) -> SciResult<OptimizationResult> {
        let n = x0.len();
        let mut x = x0.to_vec();
        let mut prev_value = f(&x);

        for iter in 0..self.config.max_iterations {
            let g = grad(&x);
            let grad_norm: SciFloat = g.iter().map(|&gi| gi * gi).sum::<SciFloat>().sqrt();

            if grad_norm < self.config.tolerance {
                return Ok(OptimizationResult {
                    x,
                    value: prev_value,
                    iterations: iter,
                    converged: true,
                    gradient_norm: grad_norm,
                });
            }

            let h = hessian(&x);

            // Solve H * delta = -g
            let delta = self.solve_linear_system(&h, &g)?;

            // Update x
            for i in 0..n {
                x[i] -= delta[i];
            }

            let value = f(&x);

            // Check for convergence
            if (value - prev_value).abs() < self.config.tolerance {
                return Ok(OptimizationResult {
                    x,
                    value,
                    iterations: iter + 1,
                    converged: true,
                    gradient_norm: grad_norm,
                });
            }

            prev_value = value;
        }

        let g = grad(&x);
        let grad_norm: SciFloat = g.iter().map(|&gi| gi * gi).sum::<SciFloat>().sqrt();

        Ok(OptimizationResult {
            x,
            value: prev_value,
            iterations: self.config.max_iterations,
            converged: false,
            gradient_norm: grad_norm,
        })
    }

    /// Solve linear system using Gaussian elimination with partial pivoting
    fn solve_linear_system(&self, a: &Hessian, b: &[SciFloat]) -> SciResult<Vec<SciFloat>> {
        let n = a.len();
        let mut a = a.clone();
        let mut b = b.to_vec();

        // Forward elimination with partial pivoting
        for i in 0..n {
            // Find pivot
            let mut max_row = i;
            let mut max_val = a[i][i].abs();
            for row in (i + 1)..n {
                if a[row][i].abs() > max_val {
                    max_val = a[row][i].abs();
                    max_row = row;
                }
            }

            if max_val < 1e-10 {
                return Err(SciError::SingularMatrix);
            }

            // Swap rows
            if max_row != i {
                a.swap(i, max_row);
                let temp = b[i];
                b[i] = b[max_row];
                b[max_row] = temp;
            }

            // Eliminate column
            for row in (i + 1)..n {
                let factor = a[row][i] / a[i][i];
                for col in i..n {
                    a[row][col] -= factor * a[i][col];
                }
                b[row] -= factor * b[i];
            }
        }

        // Back substitution
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut sum = b[i];
            for j in (i + 1)..n {
                sum -= a[i][j] * x[j];
            }
            x[i] = sum / a[i][i];
        }

        Ok(x)
    }
}

/// BFGS quasi-Newton method
pub struct BFGS {
    config: OptimizationConfig,
}

impl BFGS {
    /// Create a new BFGS optimizer
    pub fn new(config: OptimizationConfig) -> Self {
        Self { config }
    }

    /// Minimize function using BFGS
    pub fn minimize(
        &self,
        f: &ObjectiveFunction,
        grad: &GradientFunction,
        x0: &[SciFloat],
    ) -> SciResult<OptimizationResult> {
        let n = x0.len();
        let mut x = x0.to_vec();
        let mut g = grad(&x);
        let mut h = Self::initialize_hessian(n);
        let mut prev_value = f(&x);

        for iter in 0..self.config.max_iterations {
            let grad_norm: SciFloat = g.iter().map(|&gi| gi * gi).sum::<SciFloat>().sqrt();

            if grad_norm < self.config.tolerance {
                return Ok(OptimizationResult {
                    x,
                    value: prev_value,
                    iterations: iter,
                    converged: true,
                    gradient_norm: grad_norm,
                });
            }

            // Compute search direction: d = -H * g
            let mut d = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    d[i] -= h[i][j] * g[j];
                }
            }

            // Line search (simplified)
            let alpha = self.line_search(f, &x, &d, &g)?;

            let x_new: Vec<SciFloat> = x.iter().zip(d.iter()).map(|(&xi, &di)| xi + alpha * di).collect();
            let g_new = grad(&x_new);
            let value = f(&x_new);

            // Update BFGS
            let s: Vec<SciFloat> = x_new.iter().zip(x.iter()).map(|(&ni, &oi)| ni - oi).collect();
            let y: Vec<SciFloat> = g_new.iter().zip(g.iter()).map(|(&ni, &oi)| ni - oi).collect();

            let sy: SciFloat = s.iter().zip(y.iter()).map(|(&si, &yi)| si * yi).sum();

            if sy > 1e-10 {
                // Update H
                let _n_float = n as SciFloat;

                // H * y
                let mut hy = vec![0.0; n];
                for i in 0..n {
                    for j in 0..n {
                        hy[i] += h[i][j] * y[j];
                    }
                }

                let yhy: SciFloat = y.iter().zip(hy.iter()).map(|(&yi, &hyi)| yi * hyi).sum();

                for i in 0..n {
                    for j in 0..n {
                        h[i][j] += (1.0 + yhy / sy) * s[i] * s[j] / sy
                            - (hy[i] * s[j] + s[i] * hy[j]) / sy;
                    }
                }
            }

            x = x_new;
            g = g_new;

            if (value - prev_value).abs() < self.config.tolerance {
                return Ok(OptimizationResult {
                    x,
                    value,
                    iterations: iter + 1,
                    converged: true,
                    gradient_norm: grad_norm,
                });
            }

            prev_value = value;
        }

        let grad_norm: SciFloat = g.iter().map(|&gi| gi * gi).sum::<SciFloat>().sqrt();

        Ok(OptimizationResult {
            x,
            value: prev_value,
            iterations: self.config.max_iterations,
            converged: false,
            gradient_norm: grad_norm,
        })
    }

    fn initialize_hessian(n: usize) -> Hessian {
        let mut h = vec![vec![0.0; n]; n];
        for i in 0..n {
            h[i][i] = 1.0;
        }
        h
    }

    /// Simple backtracking line search
    fn line_search(
        &self,
        f: &ObjectiveFunction,
        x: &[SciFloat],
        d: &[SciFloat],
        g: &[SciFloat],
    ) -> SciResult<SciFloat> {
        let mut alpha = 1.0;
        let rho = 0.5;
        let c = 1e-4;

        let f0 = f(x);
        let dg: SciFloat = d.iter().zip(g.iter()).map(|(&di, &gi)| di * gi).sum();

        for _ in 0..20 {
            let x_new: Vec<SciFloat> = x.iter().zip(d.iter()).map(|(&xi, &di)| xi + alpha * di).collect();
            let f_new = f(&x_new);

            if f_new <= f0 + c * alpha * dg {
                return Ok(alpha);
            }

            alpha *= rho;
        }

        Ok(alpha)
    }
}

/// Conjugate gradient method
pub struct ConjugateGradient {
    config: OptimizationConfig,
}

impl ConjugateGradient {
    /// Create a new conjugate gradient optimizer
    pub fn new(config: OptimizationConfig) -> Self {
        Self { config }
    }

    /// Minimize using Fletcher-Reeves conjugate gradient
    pub fn minimize(
        &self,
        f: &ObjectiveFunction,
        grad: &GradientFunction,
        x0: &[SciFloat],
    ) -> SciResult<OptimizationResult> {
        let n = x0.len();
        let mut x = x0.to_vec();
        let mut g = grad(&x);
        let mut d: Vec<SciFloat> = g.iter().map(|&gi| -gi).collect();
        let mut prev_value = f(&x);

        for iter in 0..self.config.max_iterations {
            let grad_norm: SciFloat = g.iter().map(|&gi| gi * gi).sum::<SciFloat>().sqrt();

            if grad_norm < self.config.tolerance {
                return Ok(OptimizationResult {
                    x,
                    value: prev_value,
                    iterations: iter,
                    converged: true,
                    gradient_norm: grad_norm,
                });
            }

            // Line search
            let alpha = self.line_search(f, &x, &d)?;

            let g_old = g.clone();
            let x_new: Vec<SciFloat> = x.iter().zip(d.iter()).map(|(&xi, &di)| xi + alpha * di).collect();
            g = grad(&x_new);

            // Fletcher-Reeves beta
            let g_old_norm_sq: SciFloat = g_old.iter().map(|&gi| gi * gi).sum();
            let g_norm_sq: SciFloat = g.iter().map(|&gi| gi * gi).sum();

            let beta = if g_old_norm_sq > 1e-10 {
                g_norm_sq / g_old_norm_sq
            } else {
                0.0
            };

            // New search direction
            for i in 0..n {
                d[i] = -g[i] + beta * d[i];
            }

            x = x_new;
            let value = f(&x);

            if (value - prev_value).abs() < self.config.tolerance {
                return Ok(OptimizationResult {
                    x,
                    value,
                    iterations: iter + 1,
                    converged: true,
                    gradient_norm: grad_norm,
                });
            }

            prev_value = value;
        }

        let grad_norm: SciFloat = g.iter().map(|&gi| gi * gi).sum::<SciFloat>().sqrt();

        Ok(OptimizationResult {
            x,
            value: prev_value,
            iterations: self.config.max_iterations,
            converged: false,
            gradient_norm: grad_norm,
        })
    }

    fn line_search(&self, f: &ObjectiveFunction, x: &[SciFloat], d: &[SciFloat]) -> SciResult<SciFloat> {
        let mut alpha = 1.0;
        let rho = 0.5;

        for _ in 0..20 {
            let x_new: Vec<SciFloat> = x.iter().zip(d.iter()).map(|(&xi, &di)| xi + alpha * di).collect();
            let f_new = f(&x_new);
            let f_old = f(x);

            if f_new < f_old {
                return Ok(alpha);
            }

            alpha *= rho;
        }

        Ok(alpha)
    }
}

/// Simulated annealing for global optimization
pub struct SimulatedAnnealing {
    config: OptimizationConfig,
    initial_temp: SciFloat,
    cooling_rate: SciFloat,
}

impl SimulatedAnnealing {
    /// Create a new simulated annealing optimizer
    pub fn new(config: OptimizationConfig, initial_temp: SciFloat, cooling_rate: SciFloat) -> Self {
        Self {
            config,
            initial_temp,
            cooling_rate,
        }
    }

    /// Minimize using simulated annealing
    pub fn minimize(
        &self,
        f: &ObjectiveFunction,
        x0: &[SciFloat],
        bounds: &[(SciFloat, SciFloat)],
    ) -> SciResult<OptimizationResult> {
        let n = x0.len();
        let mut x = x0.to_vec();
        let mut current_value = f(&x);
        let mut best_x = x.clone();
        let mut best_value = current_value;
        let mut temp = self.initial_temp;

        for iter in 0..self.config.max_iterations {
            // Generate neighbor
            let mut x_new = x.clone();
            for i in 0..n {
                let range = bounds[i].1 - bounds[i].0;
                let delta = (rand_random() - 0.5) * 2.0 * range * temp / self.initial_temp;
                x_new[i] = (bounds[i].0.max(x_new[i] + delta)).min(bounds[i].1);
            }

            let new_value = f(&x_new);
            let delta = new_value - current_value;

            // Accept or reject
            if delta < 0.0 || rand_random() < (-delta / temp).exp() {
                x = x_new;
                current_value = new_value;

                if current_value < best_value {
                    best_x = x.clone();
                    best_value = current_value;
                }
            }

            // Cool down
            temp *= self.cooling_rate;

            if temp < self.config.tolerance {
                return Ok(OptimizationResult {
                    x: best_x,
                    value: best_value,
                    iterations: iter + 1,
                    converged: true,
                    gradient_norm: 0.0,
                });
            }
        }

        Ok(OptimizationResult {
            x: best_x,
            value: best_value,
            iterations: self.config.max_iterations,
            converged: false,
            gradient_norm: 0.0,
        })
    }
}

/// Simple random number generator (placeholder)
fn rand_random() -> SciFloat {
    // In a real implementation, use a proper PRNG
    // This is a simplified version for demonstration
    use core::time::Duration;
    let timestamp = Duration::from_secs(0).as_nanos() as u64;
    ((timestamp % 10000) as SciFloat) / 10000.0
}

/// Genetic algorithm for global optimization
pub struct GeneticAlgorithm {
    population_size: usize,
    mutation_rate: SciFloat,
    crossover_rate: SciFloat,
    max_generations: usize,
}

impl GeneticAlgorithm {
    /// Create a new genetic algorithm optimizer
    pub fn new(
        population_size: usize,
        mutation_rate: SciFloat,
        crossover_rate: SciFloat,
        max_generations: usize,
    ) -> Self {
        Self {
            population_size,
            mutation_rate,
            crossover_rate,
            max_generations,
        }
    }

    /// Minimize using genetic algorithm
    pub fn minimize(
        &self,
        f: &ObjectiveFunction,
        bounds: &[(SciFloat, SciFloat)],
    ) -> SciResult<OptimizationResult> {
        let _n = bounds.len();

        // Initialize population
        let mut population: Vec<Vec<SciFloat>> = (0..self.population_size)
            .map(|_| {
                bounds
                    .iter()
                    .map(|&(lower, upper)| lower + rand_random() * (upper - lower))
                    .collect()
            })
            .collect();

        let mut best_x = population[0].clone();
        let mut best_value = f(&best_x);

        for _generation in 0..self.max_generations {
            // Evaluate fitness
            let fitness: Vec<SciFloat> = population.iter().map(|x| f(x)).collect();

            // Find best
            for (i, &fit) in fitness.iter().enumerate() {
                if fit < best_value {
                    best_value = fit;
                    best_x = population[i].clone();
                }
            }

            // Selection (tournament)
            let mut new_population = Vec::new();
            while new_population.len() < self.population_size {
                let parent1 = self.tournament_select(&population, &fitness, 3);
                let parent2 = self.tournament_select(&population, &fitness, 3);

                // Crossover
                let (child1, child2) = if rand_random() < self.crossover_rate {
                    self.crossover(&parent1, &parent2)
                } else {
                    (parent1.clone(), parent2.clone())
                };

                // Mutation
                let child1 = self.mutate(child1, bounds);
                let child2 = self.mutate(child2, bounds);

                new_population.push(child1);
                if new_population.len() < self.population_size {
                    new_population.push(child2);
                }
            }

            population = new_population;
        }

        Ok(OptimizationResult {
            x: best_x,
            value: best_value,
            iterations: self.max_generations,
            converged: true,
            gradient_norm: 0.0,
        })
    }

    fn tournament_select(
        &self,
        population: &[Vec<SciFloat>],
        fitness: &[SciFloat],
        tournament_size: usize,
    ) -> Vec<SciFloat> {
        let mut best = 0;
        let mut best_fitness = fitness[0];

        for _ in 1..tournament_size {
            let idx = (rand_random() * population.len() as SciFloat) as usize;
            if fitness[idx] < best_fitness {
                best = idx;
                best_fitness = fitness[idx];
            }
        }

        population[best].clone()
    }

    fn crossover(&self, parent1: &[SciFloat], parent2: &[SciFloat]) -> (Vec<SciFloat>, Vec<SciFloat>) {
        let _n = parent1.len();
        let alpha = rand_random();

        let child1: Vec<SciFloat> = parent1
            .iter()
            .zip(parent2.iter())
            .map(|(&p1, &p2)| alpha * p1 + (1.0 - alpha) * p2)
            .collect();

        let child2: Vec<SciFloat> = parent1
            .iter()
            .zip(parent2.iter())
            .map(|(&p1, &p2)| (1.0 - alpha) * p1 + alpha * p2)
            .collect();

        (child1, child2)
    }

    fn mutate(&self, mut individual: Vec<SciFloat>, bounds: &[(SciFloat, SciFloat)]) -> Vec<SciFloat> {
        for i in 0..individual.len() {
            if rand_random() < self.mutation_rate {
                let range = bounds[i].1 - bounds[i].0;
                individual[i] += (rand_random() - 0.5) * 2.0 * range * 0.1;
                individual[i] = (bounds[i].0.max(individual[i])).min(bounds[i].1);
            }
        }
        individual
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test function: f(x) = x^2 + y^2 (minimum at (0, 0))
    fn quadratic(x: &[SciFloat]) -> SciFloat {
        x.iter().map(|&xi| xi * xi).sum()
    }

    fn quadratic_grad(x: &[SciFloat]) -> Gradient {
        x.iter().map(|&xi| 2.0 * xi).collect()
    }

    // Rosenbrock function
    fn rosenbrock(x: &[SciFloat]) -> SciFloat {
        100.0 * (x[1] - x[0] * x[0]).powi(2) + (1.0 - x[0]).powi(2)
    }

    fn rosenbrock_grad(x: &[SciFloat]) -> Gradient {
        vec![
            -400.0 * x[0] * (x[1] - x[0] * x[0]) - 2.0 * (1.0 - x[0]),
            200.0 * (x[1] - x[0] * x[0]),
        ]
    }

    #[test]
    fn test_gradient_descent() {
        let config = OptimizationConfig {
            max_iterations: 1000,
            tolerance: 1e-6,
            learning_rate: 0.1,
            ..Default::default()
        };

        let gd = GradientDescent::new(config);
        let result = gd.minimize(&quadratic, &quadratic_grad, &[5.0, 5.0]).unwrap();

        assert!(result.converged);
        assert!(result.value < 1e-4);
        assert!((result.x[0]).abs() < 0.01);
        assert!((result.x[1]).abs() < 0.01);
    }

    #[test]
    fn test_gradient_descent_with_momentum() {
        let config = OptimizationConfig {
            max_iterations: 1000,
            tolerance: 1e-6,
            learning_rate: 0.1,
            ..Default::default()
        };

        let gd = GradientDescent::new(config).with_momentum(0.9);
        let result = gd.minimize(&quadratic, &quadratic_grad, &[5.0, 5.0]).unwrap();

        assert!(result.converged);
        assert!(result.value < 1e-4);
    }

    #[test]
    fn test_newton_method() {
        let config = OptimizationConfig {
            max_iterations: 100,
            tolerance: 1e-8,
            ..Default::default()
        };

        let newton = NewtonMethod::new(config);

        // Hessian of quadratic is constant: [[2, 0], [0, 2]]
        let hessian = |_x: &[SciFloat]| -> Hessian { vec![vec![2.0, 0.0], vec![0.0, 2.0]] };

        let result = newton
            .minimize(&quadratic, &quadratic_grad, &hessian, &[5.0, 5.0])
            .unwrap();

        assert!(result.converged);
        assert!(result.value < 1e-8);
    }

    #[test]
    fn test_bfgs() {
        let config = OptimizationConfig {
            max_iterations: 1000,
            tolerance: 1e-6,
            ..Default::default()
        };

        let bfgs = BFGS::new(config);
        let result = bfgs.minimize(&rosenbrock, &rosenbrock_grad, &[-1.5, 1.0]).unwrap();

        assert!(result.converged);
        assert!(result.value < 1e-4);
    }

    #[test]
    fn test_conjugate_gradient() {
        let config = OptimizationConfig {
            max_iterations: 1000,
            tolerance: 1e-6,
            ..Default::default()
        };

        let cg = ConjugateGradient::new(config);
        let result = cg.minimize(&quadratic, &quadratic_grad, &[5.0, 5.0]).unwrap();

        assert!(result.converged);
        assert!(result.value < 1e-4);
    }

    #[test]
    fn test_simulated_annealing() {
        let config = OptimizationConfig {
            max_iterations: 1000,
            tolerance: 1e-3,
            ..Default::default()
        };

        let sa = SimulatedAnnealing::new(config, 100.0, 0.95);
        let bounds = [(-5.0, 5.0), (-5.0, 5.0)];

        let result = sa.minimize(&quadratic, &[2.0, 2.0], &bounds).unwrap();

        assert!(result.converged);
        assert!(result.value < 1.0);
    }

    #[test]
    fn test_genetic_algorithm() {
        let ga = GeneticAlgorithm::new(100, 0.1, 0.8, 100);
        let bounds = [(-5.0, 5.0), (-5.0, 5.0)];

        let result = ga.minimize(&quadratic, &bounds).unwrap();

        assert!(result.value < 1.0);
    }
}
