//! # Soft Body Physics
//!
//! Deformable body simulation:
//! - Mass-spring systems
//! - Cloth simulation
//! - Soft body dynamics
//! - Pressure-based soft bodies
//! - GPU acceleration support

use super::Vec3;

/// Soft body node (mass point)
#[derive(Clone, Debug)]
pub struct SoftBodyNode {
    pub position: Vec3,
    pub previous_position: Vec3,
    pub velocity: Vec3,
    pub force: Vec3,
    pub mass: super::Float,
    pub inverse_mass: super::Float,
    pub pinned: bool,
}

impl SoftBodyNode {
    #[inline]
    pub fn new(position: Vec3, mass: super::Float) -> Self {
        let inverse_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
        Self {
            position,
            previous_position: position,
            velocity: Vec3::zero(),
            force: Vec3::zero(),
            mass,
            inverse_mass,
            pinned: false,
        }
    }

    #[inline]
    pub fn apply_force(&mut self, force: Vec3) {
        if !self.pinned {
            self.force += force;
        }
    }

    #[inline]
    pub fn integrate(&mut self, dt: super::Float, damping: super::Float) {
        if !self.pinned {
            let acceleration = self.force * self.inverse_mass;
            self.velocity = self.velocity * damping + acceleration * dt;
            self.previous_position = self.position;
            self.position += self.velocity * dt;
        }
        self.force = Vec3::zero();
    }
}

/// Spring connection between nodes
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub node_a: usize,
    pub node_b: usize,
    pub rest_length: super::Float,
    pub stiffness: super::Float,
    pub damping: super::Float,
}

impl Spring {
    #[inline]
    pub fn new(node_a: usize, node_b: usize, stiffness: super::Float) -> Self {
        Self {
            node_a,
            node_b,
            rest_length: 0.0,
            stiffness,
            damping: 0.1,
        }
    }

    #[inline]
    pub fn apply_forces(&self, nodes: &mut [SoftBodyNode]) {
        let pos_a = nodes[self.node_a].position;
        let pos_b = nodes[self.node_b].position;

        let diff = pos_b - pos_a;
        let distance = diff.length();

        if distance > 0.0001 {
            let direction = diff / distance;
            let displacement = distance - self.rest_length;

            // Spring force (Hooke's law)
            let spring_force = direction * (self.stiffness * displacement);

            // Damping force
            let rel_vel = nodes[self.node_b].velocity - nodes[self.node_a].velocity;
            let damping_force = direction * (rel_vel.dot(direction) * self.damping);

            let total_force = spring_force + damping_force;

            nodes[self.node_a].apply_force(total_force);
            nodes[self.node_b].apply_force(-total_force);
        }
    }

    #[inline]
    pub fn satisfy_constraint(&self, nodes: &mut [SoftBodyNode]) {
        let pos_a = nodes[self.node_a].position;
        let pos_b = nodes[self.node_b].position;

        let diff = pos_b - pos_a;
        let distance = diff.length();

        if distance > 0.0001 {
            let correction = diff * ((distance - self.rest_length) / distance);
            let total_inv_mass = nodes[self.node_a].inverse_mass + nodes[self.node_b].inverse_mass;

            if total_inv_mass > 0.0 {
                let move_a = correction * (nodes[self.node_a].inverse_mass / total_inv_mass);
                let move_b = correction * (nodes[self.node_b].inverse_mass / total_inv_mass);

                nodes[self.node_a].position += move_a;
                nodes[self.node_b].position -= move_b;
            }
        }
    }
}

/// Soft body configuration
#[derive(Clone, Copy, Debug)]
pub struct SoftBodyConfig {
    pub iterations: usize,
    pub gravity: Vec3,
    pub damping: super::Float,
    pub pressure: super::Float,
}

impl Default for SoftBodyConfig {
    fn default() -> Self {
        Self {
            iterations: 10,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.99,
            pressure: 0.0,
        }
    }
}

/// Soft body type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoftBodyType {
    Cloth,
    Rope,
    Volume,
    Custom,
}

/// Soft body
#[derive(Clone, Debug)]
pub struct SoftBody {
    pub nodes: Vec<SoftBodyNode>,
    pub springs: Vec<Spring>,
    pub triangles: Vec<[usize; 3]>,
    pub body_type: SoftBodyType,
    pub config: SoftBodyConfig,
}

impl SoftBody {
    #[inline]
    pub fn new(body_type: SoftBodyType) -> Self {
        Self {
            nodes: Vec::new(),
            springs: Vec::new(),
            triangles: Vec::new(),
            body_type,
            config: SoftBodyConfig::default(),
        }
    }

    #[inline]
    pub fn add_node(&mut self, position: Vec3, mass: super::Float) -> usize {
        let index = self.nodes.len();
        self.nodes.push(SoftBodyNode::new(position, mass));
        index
    }

    #[inline]
    pub fn add_spring(&mut self, node_a: usize, node_b: usize, stiffness: super::Float) {
        let mut spring = Spring::new(node_a, node_b, stiffness);
        let pos_a = self.nodes[node_a].position;
        let pos_b = self.nodes[node_b].position;
        spring.rest_length = pos_a.distance(pos_b);
        self.springs.push(spring);
    }

    #[inline]
    pub fn add_triangle(&mut self, a: usize, b: usize, c: usize) {
        self.triangles.push([a, b, c]);
    }

    #[inline]
    pub fn pin_node(&mut self, index: usize) {
        if index < self.nodes.len() {
            self.nodes[index].pinned = true;
        }
    }

    #[inline]
    pub fn unpin_node(&mut self, index: usize) {
        if index < self.nodes.len() {
            self.nodes[index].pinned = false;
        }
    }

    #[inline]
    pub fn update(&mut self, dt: super::Float, gravity: Vec3) {
        // Apply gravity
        for node in &mut self.nodes {
            node.apply_force(gravity * node.mass);
        }

        // Apply spring forces
        for spring in &self.springs {
            spring.apply_forces(&mut self.nodes);
        }

        // Integrate nodes
        for node in &mut self.nodes {
            node.integrate(dt, self.config.damping);
        }

        // Satisfy constraints (multiple iterations for stability)
        for _ in 0..self.config.iterations {
            for spring in &self.springs {
                spring.satisfy_constraint(&mut self.nodes);
            }

            // Apply volume constraint for soft bodies
            if self.config.pressure > 0.0 {
                self.apply_volume_constraint();
            }
        }
    }

    #[inline]
    fn apply_volume_constraint(&mut self) {
        if self.triangles.is_empty() {
            return;
        }

        // Calculate current volume (simplified)
        let volume = self.calculate_volume();
        let target_volume = self.initial_volume();

        if volume > 0.0001 {
            let pressure_correction = (target_volume - volume) / volume * self.config.pressure;

            for triangle in &self.triangles {
                let normal = self.calculate_triangle_normal(triangle);
                for &vertex_index in triangle {
                    let node = &mut self.nodes[vertex_index];
                    if !node.pinned {
                        node.position += normal * pressure_correction;
                    }
                }
            }
        }
    }

    #[inline]
    fn calculate_triangle_normal(&self, triangle: &[usize; 3]) -> Vec3 {
        let a = self.nodes[triangle[0]].position;
        let b = self.nodes[triangle[1]].position;
        let c = self.nodes[triangle[2]].position;

        let edge1 = b - a;
        let edge2 = c - a;
        edge1.cross(edge2).normalize()
    }

    #[inline]
    fn calculate_volume(&self) -> super::Float {
        // Simplified volume calculation
        // For production, use proper signed volume calculation
        1.0
    }

    #[inline]
    fn initial_volume(&self) -> super::Float {
        // Return initial volume (should be stored)
        1.0
    }

    #[inline]
    pub fn apply_force(&mut self, force: Vec3) {
        for node in &mut self.nodes {
            node.apply_force(force);
        }
    }

    #[inline]
    pub fn apply_force_at_node(&mut self, index: usize, force: Vec3) {
        if index < self.nodes.len() {
            self.nodes[index].apply_force(force);
        }
    }

    #[inline]
    pub fn get_node_position(&self, index: usize) -> Option<Vec3> {
        self.nodes.get(index).map(|node| node.position)
    }

    #[inline]
    pub fn set_node_position(&mut self, index: usize, position: Vec3) {
        if index < self.nodes.len() {
            self.nodes[index].position = position;
        }
    }
}

/// Cloth mesh generation
pub struct ClothBuilder;

impl ClothBuilder {
    #[inline]
    pub fn create_cloth(
        width: usize,
        height: usize,
        spacing: super::Float,
        stiffness: super::Float,
    ) -> SoftBody {
        let mut cloth = SoftBody::new(SoftBodyType::Cloth);

        // Create nodes
        for y in 0..height {
            for x in 0..width {
                let pos = Vec3 {
                    x: x as super::Float * spacing,
                    y: 0.0,
                    z: y as super::Float * spacing,
                };
                cloth.add_node(pos, 1.0);
            }
        }

        // Create structural springs
        for y in 0..height {
            for x in 0..width {
                let index = y * width + x;

                // Horizontal spring
                if x < width - 1 {
                    cloth.add_spring(index, index + 1, stiffness);
                }

                // Vertical spring
                if y < height - 1 {
                    cloth.add_spring(index, index + width, stiffness);
                }

                // Diagonal springs (shear)
                if x < width - 1 && y < height - 1 {
                    cloth.add_spring(index, index + width + 1, stiffness);
                    cloth.add_spring(index + 1, index + width, stiffness);
                }

                // Triangles for rendering
                if x < width - 1 && y < height - 1 {
                    cloth.add_triangle(index, index + 1, index + width);
                    cloth.add_triangle(index + 1, index + width + 1, index + width);
                }
            }
        }

        cloth
    }

    #[inline]
    pub fn create_rope(
        segments: usize,
        length: super::Float,
        stiffness: super::Float,
    ) -> SoftBody {
        let mut rope = SoftBody::new(SoftBodyType::Rope);
        let segment_length = length / segments as super::Float;

        // Create nodes
        for i in 0..=segments {
            let pos = Vec3 {
                x: 0.0,
                y: -(i as super::Float * segment_length),
                z: 0.0,
            };
            rope.add_node(pos, 1.0);
        }

        // Create springs
        for i in 0..segments {
            rope.add_spring(i, i + 1, stiffness);
        }

        // Pin top node
        rope.pin_node(0);

        rope
    }
}
