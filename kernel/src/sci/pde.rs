//! # Partial Differential Equation Solvers
//!
//! Numerical methods for solving PDEs:
//! - Finite Difference Method (FDM)
//! - Finite Element Method basics
//! - Heat equation
//! - Wave equation
//! - Boundary condition handling

use crate::sci::{SciError, SciFloat, SciResult};
use alloc::vec::Vec;

/// 2D Grid for PDE solving
#[derive(Debug, Clone)]
pub struct Grid2D {
    /// Number of grid points in x direction
    pub nx: usize,
    /// Number of grid points in y direction
    pub ny: usize,
    /// Grid spacing in x
    pub dx: SciFloat,
    /// Grid spacing in y
    pub dy: SciFloat,
    /// Domain extent in x
    pub x_min: SciFloat,
    pub x_max: SciFloat,
    /// Domain extent in y
    pub y_min: SciFloat,
    pub y_max: SciFloat,
    /// Data values
    data: Vec<Vec<SciFloat>>,
}

impl Grid2D {
    /// Create a new 2D grid
    pub fn new(
        nx: usize,
        ny: usize,
        x_min: SciFloat,
        x_max: SciFloat,
        y_min: SciFloat,
        y_max: SciFloat,
    ) -> Self {
        let dx = (x_max - x_min) / (nx - 1) as SciFloat;
        let dy = (y_max - y_min) / (ny - 1) as SciFloat;

        Self {
            nx,
            ny,
            dx,
            dy,
            x_min,
            x_max,
            y_min,
            y_max,
            data: vec![vec![0.0; ny]; nx],
        }
    }

    /// Get grid point (i, j) corresponds to (x[i], y[j])
    pub fn get(&self, i: usize, j: usize) -> SciFloat {
        assert!(i < self.nx && j < self.ny);
        self.data[i][j]
    }

    /// Set grid point
    pub fn set(&mut self, i: usize, j: usize, value: SciFloat) {
        assert!(i < self.nx && j < self.ny);
        self.data[i][j] = value;
    }

    /// Get x-coordinate at index i
    pub fn x(&self, i: usize) -> SciFloat {
        self.x_min + i as SciFloat * self.dx
    }

    /// Get y-coordinate at index j
    pub fn y(&self, j: usize) -> SciFloat {
        self.y_min + j as SciFloat * self.dy
    }

    /// Get underlying data
    pub fn data(&self) -> &Vec<Vec<SciFloat>> {
        &self.data
    }

    /// Get mutable data
    pub fn data_mut(&mut self) -> &mut Vec<Vec<SciFloat>> {
        &mut self.data
    }

    /// Clone data
    pub fn clone_data(&self) -> Vec<Vec<SciFloat>> {
        self.data.clone()
    }
}

/// Boundary condition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryType {
    Dirichlet,  // Fixed value
    Neumann,    // Fixed derivative
    Periodic,   // Periodic boundary
}

/// Boundary condition specification
#[derive(Debug, Clone)]
pub struct BoundaryCondition {
    pub boundary_type: BoundaryType,
    pub value: SciFloat,
}

impl BoundaryCondition {
    /// Create Dirichlet boundary condition
    pub fn dirichlet(value: SciFloat) -> Self {
        Self {
            boundary_type: BoundaryType::Dirichlet,
            value,
        }
    }

    /// Create Neumann boundary condition
    pub fn neumann(value: SciFloat) -> Self {
        Self {
            boundary_type: BoundaryType::Neumann,
            value,
        }
    }

    /// Create periodic boundary condition
    pub fn periodic() -> Self {
        Self {
            boundary_type: BoundaryType::Periodic,
            value: 0.0,
        }
    }
}

/// Finite difference solver for 1D heat equation
///
/// ∂u/∂t = α ∂²u/∂x²
pub struct HeatEquation1D {
    /// Thermal diffusivity
    pub alpha: SciFloat,
    /// Time step
    pub dt: SciFloat,
}

impl HeatEquation1D {
    /// Create new heat equation solver
    pub fn new(alpha: SciFloat, dt: SciFloat) -> Self {
        Self { alpha, dt }
    }

    /// Solve using explicit Euler (forward in time, centered in space)
    ///
    /// Stability condition: dt <= dx² / (2α)
    pub fn solve_explicit(
        &self,
        u0: &[SciFloat],
        x_span: (SciFloat, SciFloat),
        t_span: (SciFloat, SciFloat),
        bc_left: &BoundaryCondition,
        bc_right: &BoundaryCondition,
    ) -> SciResult<(Vec<SciFloat>, Vec<SciFloat>, Vec<Vec<SciFloat>>)> {
        let n = u0.len();
        let dx = (x_span.1 - x_span.0) / (n - 1) as SciFloat;

        // Stability check
        let r = self.alpha * self.dt / (dx * dx);
        if r > 0.5 {
            return Err(SciError::InvalidParameters);
        }

        let num_steps = ((t_span.1 - t_span.0) / self.dt).ceil() as usize;

        let mut u = u0.to_vec();
        let mut u_history = Vec::new();
        let mut t_history = Vec::new();
        let x: Vec<SciFloat> = (0..n).map(|i| x_span.0 + i as SciFloat * dx).collect();

        u_history.push(u.clone());
        t_history.push(t_span.0);

        for step in 0..num_steps {
            let mut u_new = u.clone();

            // Interior points
            for i in 1..n - 1 {
                u_new[i] = u[i] + r * (u[i + 1] - 2.0 * u[i] + u[i - 1]);
            }

            // Boundary conditions
            match bc_left.boundary_type {
                BoundaryType::Dirichlet => u_new[0] = bc_left.value,
                BoundaryType::Neumann => {
                    u_new[0] = u_new[1] - bc_left.value * dx;
                }
                BoundaryType::Periodic => u_new[0] = u_new[n - 2],
            }

            match bc_right.boundary_type {
                BoundaryType::Dirichlet => u_new[n - 1] = bc_right.value,
                BoundaryType::Neumann => {
                    u_new[n - 1] = u_new[n - 2] + bc_right.value * dx;
                }
                BoundaryType::Periodic => u_new[n - 1] = u_new[1],
            }

            u = u_new;

            if step % 10 == 0 {
                u_history.push(u.clone());
                t_history.push(t_span.0 + (step + 1) as SciFloat * self.dt);
            }
        }

        Ok((x, t_history, u_history))
    }

    /// Solve using Crank-Nicolson method (implicit, unconditionally stable)
    pub fn solve_implicit(
        &self,
        u0: &[SciFloat],
        x_span: (SciFloat, SciFloat),
        t_span: (SciFloat, SciFloat),
        bc_left: &BoundaryCondition,
        bc_right: &BoundaryCondition,
    ) -> SciResult<(Vec<SciFloat>, Vec<SciFloat>, Vec<Vec<SciFloat>>)> {
        let n = u0.len();
        let dx = (x_span.1 - x_span.0) / (n - 1) as SciFloat;
        let r = self.alpha * self.dt / (2.0 * dx * dx);

        let num_steps = ((t_span.1 - t_span.0) / self.dt).ceil() as usize;

        let mut u = u0.to_vec();
        let mut u_history = Vec::new();
        let mut t_history = Vec::new();
        let x: Vec<SciFloat> = (0..n).map(|i| x_span.0 + i as SciFloat * dx).collect();

        u_history.push(u.clone());
        t_history.push(t_span.0);

        // Tridiagonal matrix coefficients
        for _step in 0..num_steps {
            // Build system
            let mut a = vec![0.0; n];
            let mut b = vec![0.0; n];
            let mut c = vec![0.0; n];
            let mut d = u.clone();

            for i in 1..n - 1 {
                a[i] = -r;
                b[i] = 1.0 + 2.0 * r;
                c[i] = -r;
                d[i] = r * u[i - 1] + (1.0 - 2.0 * r) * u[i] + r * u[i + 1];
            }

            // Boundary conditions
            match bc_left.boundary_type {
                BoundaryType::Dirichlet => {
                    b[0] = 1.0;
                    c[0] = 0.0;
                    d[0] = bc_left.value;
                }
                BoundaryType::Neumann => {
                    b[0] = 1.0;
                    c[0] = -1.0;
                    d[0] = bc_left.value * dx;
                }
                _ => {}
            }

            match bc_right.boundary_type {
                BoundaryType::Dirichlet => {
                    a[n - 1] = 0.0;
                    b[n - 1] = 1.0;
                    d[n - 1] = bc_right.value;
                }
                BoundaryType::Neumann => {
                    a[n - 1] = -1.0;
                    b[n - 1] = 1.0;
                    d[n - 1] = bc_right.value * dx;
                }
                _ => {}
            }

            // Solve tridiagonal system (Thomas algorithm)
            let u_new = self.solve_thomas(&a, &b, &c, &d)?;
            u = u_new;

            if _step % 10 == 0 {
                u_history.push(u.clone());
                t_history.push(t_span.0 + (_step + 1) as SciFloat * self.dt);
            }
        }

        Ok((x, t_history, u_history))
    }

    /// Thomas algorithm for tridiagonal systems
    fn solve_thomas(
        &self,
        a: &[SciFloat],
        b: &[SciFloat],
        c: &[SciFloat],
        d: &[SciFloat],
    ) -> SciResult<Vec<SciFloat>> {
        let n = a.len();

        let mut c_prime = vec![0.0; n];
        let mut d_prime = vec![0.0; n];

        // Forward elimination
        c_prime[0] = if b[0].abs() > 1e-10 {
            c[0] / b[0]
        } else {
            0.0
        };
        d_prime[0] = if b[0].abs() > 1e-10 {
            d[0] / b[0]
        } else {
            d[0]
        };

        for i in 1..n {
            let denom = b[i] - a[i] * c_prime[i - 1];
            if denom.abs() < 1e-10 {
                return Err(SciError::SingularMatrix);
            }

            c_prime[i] = c[i] / denom;
            d_prime[i] = (d[i] - a[i] * d_prime[i - 1]) / denom;
        }

        // Back substitution
        let mut x = vec![0.0; n];
        x[n - 1] = d_prime[n - 1];

        for i in (0..n - 1).rev() {
            x[i] = d_prime[i] - c_prime[i] * x[i + 1];
        }

        Ok(x)
    }
}

/// Finite difference solver for 1D wave equation
///
/// ∂²u/∂t² = c² ∂²u/∂x²
pub struct WaveEquation1D {
    /// Wave speed
    pub c: SciFloat,
    /// Time step
    pub dt: SciFloat,
}

impl WaveEquation1D {
    /// Create new wave equation solver
    pub fn new(c: SciFloat, dt: SciFloat) -> Self {
        Self { c, dt }
    }

    /// Solve using explicit finite difference
    ///
    /// Stability condition: CFL <= 1, where CFL = c * dt / dx
    pub fn solve(
        &self,
        u0: &[SciFloat],           // Initial displacement
        v0: &[SciFloat],           // Initial velocity
        x_span: (SciFloat, SciFloat),
        t_span: (SciFloat, SciFloat),
        bc_left: &BoundaryCondition,
        bc_right: &BoundaryCondition,
    ) -> SciResult<(Vec<SciFloat>, Vec<SciFloat>, Vec<Vec<SciFloat>>)> {
        let n = u0.len();
        let dx = (x_span.1 - x_span.0) / (n - 1) as SciFloat;

        // CFL condition
        let cfl = self.c * self.dt / dx;
        if cfl > 1.0 {
            return Err(SciError::InvalidParameters);
        }

        let num_steps = ((t_span.1 - t_span.0) / self.dt).ceil() as usize;

        let mut u_prev = u0.to_vec();
        let mut u = u0.to_vec();
        let mut u_next = vec![0.0; n];

        let x: Vec<SciFloat> = (0..n).map(|i| x_span.0 + i as SciFloat * dx).collect();
        let mut u_history = Vec::new();
        let mut t_history = Vec::new();

        u_history.push(u.clone());
        t_history.push(t_span.0);

        // First time step (use initial velocity)
        for i in 1..n - 1 {
            u_next[i] = u[i]
                + 0.5 * cfl * cfl * (u[i + 1] - 2.0 * u[i] + u[i - 1])
                + self.dt * v0[i];
        }

        // Apply boundary conditions
        self.apply_bcs(&mut u_next, bc_left, bc_right);

        u_prev = u;
        u = u_next.clone();

        for step in 1..num_steps {
            // Interior points
            for i in 1..n - 1 {
                u_next[i] = 2.0 * u[i] - u_prev[i] + cfl * cfl * (u[i + 1] - 2.0 * u[i] + u[i - 1]);
            }

            // Boundary conditions
            self.apply_bcs(&mut u_next, bc_left, bc_right);

            u_prev = u.clone();
            u = u_next.clone();

            if step % 10 == 0 {
                u_history.push(u.clone());
                t_history.push(t_span.0 + (step + 1) as SciFloat * self.dt);
            }
        }

        Ok((x, t_history, u_history))
    }

    fn apply_bcs(&self, u: &mut [SciFloat], bc_left: &BoundaryCondition, bc_right: &BoundaryCondition) {
        let n = u.len();

        match bc_left.boundary_type {
            BoundaryType::Dirichlet => u[0] = bc_left.value,
            BoundaryType::Neumann => u[0] = u[1],
            BoundaryType::Periodic => u[0] = u[n - 2],
        }

        match bc_right.boundary_type {
            BoundaryType::Dirichlet => u[n - 1] = bc_right.value,
            BoundaryType::Neumann => u[n - 1] = u[n - 2],
            BoundaryType::Periodic => u[n - 1] = u[1],
        }
    }
}

/// 2D Heat equation solver
///
/// ∂u/∂t = α (∂²u/∂x² + ∂²u/∂y²)
pub struct HeatEquation2D {
    /// Thermal diffusivity
    pub alpha: SciFloat,
    /// Time step
    pub dt: SciFloat,
}

impl HeatEquation2D {
    /// Create new 2D heat equation solver
    pub fn new(alpha: SciFloat, dt: SciFloat) -> Self {
        Self { alpha, dt }
    }

    /// Solve 2D heat equation using explicit method
    pub fn solve_explicit(
        &self,
        grid: &mut Grid2D,
        t_span: (SciFloat, SciFloat),
        bc: &BoundaryCondition2D,
    ) -> SciResult<Vec<Grid2D>> {
        let dx = grid.dx;
        let dy = grid.dy;
        let nx = grid.nx;
        let ny = grid.ny;

        // Stability condition
        let rx = self.alpha * self.dt / (dx * dx);
        let ry = self.alpha * self.dt / (dy * dy);

        if rx + ry > 0.5 {
            return Err(SciError::InvalidParameters);
        }

        let num_steps = ((t_span.1 - t_span.0) / self.dt).ceil() as usize;
        let mut history = Vec::new();

        for step in 0..num_steps {
            let mut new_data = grid.clone_data();

            // Interior points
            for i in 1..nx - 1 {
                for j in 1..ny - 1 {
                    let u = grid.get(i, j);
                    new_data[i][j] = u
                        + rx * (grid.get(i + 1, j) - 2.0 * u + grid.get(i - 1, j))
                        + ry * (grid.get(i, j + 1) - 2.0 * u + grid.get(i, j - 1));
                }
            }

            // Apply boundary conditions
            self.apply_bcs_2d(&mut new_data, grid, bc);

            *grid.data_mut() = new_data;

            if step % 10 == 0 {
                history.push(grid.clone());
            }
        }

        Ok(history)
    }

    fn apply_bcs_2d(
        &self,
        data: &mut [Vec<SciFloat>],
        grid: &Grid2D,
        bc: &BoundaryCondition2D,
    ) {
        let nx = grid.nx;
        let ny = grid.ny;

        // Left boundary
        for j in 0..ny {
            match bc.left.boundary_type {
                BoundaryType::Dirichlet => data[0][j] = bc.left.value,
                BoundaryType::Neumann => data[0][j] = data[1][j],
                BoundaryType::Periodic => data[0][j] = data[nx - 2][j],
            }
        }

        // Right boundary
        for j in 0..ny {
            match bc.right.boundary_type {
                BoundaryType::Dirichlet => data[nx - 1][j] = bc.right.value,
                BoundaryType::Neumann => data[nx - 1][j] = data[nx - 2][j],
                BoundaryType::Periodic => data[nx - 1][j] = data[1][j],
            }
        }

        // Bottom boundary
        for i in 0..nx {
            match bc.bottom.boundary_type {
                BoundaryType::Dirichlet => data[i][0] = bc.bottom.value,
                BoundaryType::Neumann => data[i][0] = data[i][1],
                BoundaryType::Periodic => data[i][0] = data[i][ny - 2],
            }
        }

        // Top boundary
        for i in 0..nx {
            match bc.top.boundary_type {
                BoundaryType::Dirichlet => data[i][ny - 1] = bc.top.value,
                BoundaryType::Neumann => data[i][ny - 1] = data[i][ny - 2],
                BoundaryType::Periodic => data[i][ny - 1] = data[i][1],
            }
        }
    }
}

/// Boundary conditions for 2D problems
#[derive(Debug, Clone)]
pub struct BoundaryCondition2D {
    pub left: BoundaryCondition,
    pub right: BoundaryCondition,
    pub bottom: BoundaryCondition,
    pub top: BoundaryCondition,
}

/// Simple 2D finite element solver (simplified)
///
/// Solves Poisson equation: -∇²u = f
pub struct PoissonSolver2D;

impl PoissonSolver2D {
    /// Solve using 5-point stencil (finite difference)
    pub fn solve(
        grid: &mut Grid2D,
        f: &dyn Fn(SciFloat, SciFloat) -> SciFloat,
        bc: &BoundaryCondition2D,
        max_iter: usize,
        tol: SciFloat,
    ) -> SciResult<usize> {
        let nx = grid.nx;
        let ny = grid.ny;
        let dx = grid.dx;
        let dy = grid.dy;

        let mut iterations = 0;

        for iter in 0..max_iter {
            let mut max_diff = 0.0;

            // Interior points (Gauss-Seidel iteration)
            for i in 1..nx - 1 {
                for j in 1..ny - 1 {
                    let old = grid.get(i, j);

                    let x = grid.x(i);
                    let y = grid.y(j);

                    let numerator = (grid.get(i + 1, j) + grid.get(i - 1, j)) * dy * dy
                        + (grid.get(i, j + 1) + grid.get(i, j - 1)) * dx * dx
                        + f(x, y) * dx * dx * dy * dy;

                    let denominator = 2.0 * (dx * dx + dy * dy);
                    let new_val = numerator / denominator;

                    grid.set(i, j, new_val);

                    let diff = (new_val - old).abs();
                    if diff > max_diff {
                        max_diff = diff;
                    }
                }
            }

            // Apply boundary conditions
            Self::apply_bcs_poisson(grid, bc);

            iterations = iter + 1;

            if max_diff < tol {
                break;
            }
        }

        Ok(iterations)
    }

    fn apply_bcs_poisson(grid: &mut Grid2D, bc: &BoundaryCondition2D) {
        let nx = grid.nx;
        let ny = grid.ny;

        for j in 0..ny {
            if let BoundaryType::Dirichlet = bc.left.boundary_type {
                grid.set(0, j, bc.left.value);
            }
            if let BoundaryType::Dirichlet = bc.right.boundary_type {
                grid.set(nx - 1, j, bc.right.value);
            }
        }

        for i in 0..nx {
            if let BoundaryType::Dirichlet = bc.bottom.boundary_type {
                grid.set(i, 0, bc.bottom.value);
            }
            if let BoundaryType::Dirichlet = bc.top.boundary_type {
                grid.set(i, ny - 1, bc.top.value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid2d() {
        let grid = Grid2D::new(10, 10, 0.0, 1.0, 0.0, 1.0);

        assert_eq!(grid.nx, 10);
        assert_eq!(grid.ny, 10);
        assert!((grid.dx - 0.111...).abs() < 0.01);
        assert!((grid.dy - 0.111...).abs() < 0.01);

        grid.set(5, 5, 42.0);
        assert_eq!(grid.get(5, 5), 42.0);
    }

    #[test]
    fn test_heat_equation_1d_explicit() {
        let solver = HeatEquation1D::new(1.0, 0.001);

        let n = 101;
        let u0: Vec<SciFloat> = (0..n)
            .map(|i| {
                let x = i as SciFloat / (n - 1) as SciFloat;
                if x >= 0.4 && x <= 0.6 {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();

        let bc_left = BoundaryCondition::dirichlet(0.0);
        let bc_right = BoundaryCondition::dirichlet(0.0);

        let result = solver
            .solve_explicit(&u0, (0.0, 1.0), (0.0, 0.1), &bc_left, &bc_right)
            .unwrap();

        assert!(!result.0.is_empty());
        assert!(!result.1.is_empty());
        assert!(!result.2.is_empty());
    }

    #[test]
    fn test_heat_equation_1d_implicit() {
        let solver = HeatEquation1D::new(1.0, 0.01);

        let n = 51;
        let u0: Vec<SciFloat> = (0..n)
            .map(|i| {
                let x = i as SciFloat / (n - 1) as SciFloat;
                (core::f64::consts::PI * x).sin()
            })
            .collect();

        let bc_left = BoundaryCondition::dirichlet(0.0);
        let bc_right = BoundaryCondition::dirichlet(0.0);

        let result = solver
            .solve_implicit(&u0, (0.0, 1.0), (0.0, 0.1), &bc_left, &bc_right)
            .unwrap();

        assert!(!result.0.is_empty());
        assert!(!result.1.is_empty());
    }

    #[test]
    fn test_wave_equation_1d() {
        let solver = WaveEquation1D::new(1.0, 0.001);

        let n = 101;
        let u0: Vec<SciFloat> = (0..n)
            .map(|i| {
                let x = i as SciFloat / (n - 1) as SciFloat;
                (core::f64::consts::PI * x).sin()
            })
            .collect();

        let v0 = vec![0.0; n];

        let bc_left = BoundaryCondition::dirichlet(0.0);
        let bc_right = BoundaryCondition::dirichlet(0.0);

        let result = solver
            .solve(&u0, &v0, (0.0, 1.0), (0.0, 1.0), &bc_left, &bc_right)
            .unwrap();

        assert!(!result.0.is_empty());
        assert!(!result.1.is_empty());
    }

    #[test]
    fn test_poisson_solver() {
        let mut grid = Grid2D::new(21, 21, 0.0, 1.0, 0.0, 1.0);

        // Source term
        let f = |_x: SciFloat, _y: SciFloat| -> SciFloat { 1.0 };

        // Boundary conditions
        let bc = BoundaryCondition2D {
            left: BoundaryCondition::dirichlet(0.0),
            right: BoundaryCondition::dirichlet(0.0),
            bottom: BoundaryCondition::dirichlet(0.0),
            top: BoundaryCondition::dirichlet(0.0),
        };

        let iterations = PoissonSolver2D::solve(&mut grid, &f, &bc, 1000, 1e-6).unwrap();

        assert!(iterations > 0);
        assert!(iterations <= 1000);

        // Check that solution is positive
        let max_val = grid
            .data()
            .iter()
            .map(|row| row.iter().cloned().fold(0.0, SciFloat::max))
            .fold(0.0, SciFloat::max);

        assert!(max_val > 0.0);
    }

    #[test]
    fn test_boundary_conditions() {
        let dirichlet = BoundaryCondition::dirichlet(1.0);
        assert_eq!(dirichlet.boundary_type, BoundaryType::Dirichlet);
        assert_eq!(dirichlet.value, 1.0);

        let neumann = BoundaryCondition::neumann(0.5);
        assert_eq!(neumann.boundary_type, BoundaryType::Neumann);

        let periodic = BoundaryCondition::periodic();
        assert_eq!(periodic.boundary_type, BoundaryType::Periodic);
    }
}
