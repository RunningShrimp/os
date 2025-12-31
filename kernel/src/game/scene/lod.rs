//! # Level of Detail (LOD)
//!
//! LOD system for optimizing rendering at distance.

use alloc::vec::Vec;
use alloc::string::String;
use crate::compat::Float;
use super::LODConfig;

/// LOD component
#[derive(Clone, Debug)]
pub struct LODComponent {
    pub config: LODConfig,
    pub current_level: usize,
    pub transition_time: Float,
}

impl LODComponent {
    #[inline]
    pub fn new(config: LODConfig) -> Self {
        Self {
            config,
            current_level: 0,
            transition_time: 0.0,
        }
    }

    #[inline]
    pub fn update(&mut self, distance: Float, dt: Float) -> usize {
        let new_level = self.config.select_level(distance);

        if new_level != self.current_level {
            self.transition_time += dt;
        }

        self.current_level = new_level;
        self.current_level
    }

    #[inline]
    pub fn get_current_level(&self) -> usize {
        self.current_level
    }

    #[inline]
    pub fn get_level_config(&self) -> Option<&super::LODLevel> {
        self.config.get_level(self.current_level)
    }

    #[inline]
    pub fn is_transitioning(&self, threshold: Float) -> bool {
        self.transition_time < threshold
    }
}
