//! # Animation Blend Trees
//!
//! Blend trees for mixing animations based on parameters.

use alloc::vec::Vec;
use alloc::string::String;
use crate::compat::Float;

/// Blend tree node
#[derive(Clone, Debug)]
pub enum BlendTreeNode {
    Leaf {
        clip: String,
        threshold: Float,
    },
    Linear1D {
        parameter: String,
        children: Vec<BlendTreeNode>,
    },
    SimpleDirectional2D {
        param_x: String,
        param_y: String,
        children: Vec<BlendTreeNode>,
    },
    FreeformDirectional2D {
        param_x: String,
        param_y: String,
        children: Vec<BlendTreeNode>,
    },
}

impl BlendTreeNode {
    #[inline]
    pub fn leaf(clip: String, threshold: Float) -> Self {
        Self::Leaf { clip, threshold }
    }

    #[inline]
    pub fn linear_1d(parameter: String) -> Self {
        Self::Linear1D {
            parameter,
            children: Vec::new(),
        }
    }

    #[inline]
    pub fn add_child(&mut self, child: BlendTreeNode) {
        match self {
            Self::Linear1D { children, .. } => children.push(child),
            Self::SimpleDirectional2D { children, .. } => children.push(child),
            Self::FreeformDirectional2D { children, .. } => children.push(child),
            _ => {}
        }
    }
}

/// Blend tree
#[derive(Clone, Debug)]
pub struct BlendTree {
    pub root: BlendTreeNode,
    pub parameters: Vec<(String, Float)>,
}

impl BlendTree {
    #[inline]
    pub fn new(root: BlendTreeNode) -> Self {
        Self {
            root,
            parameters: Vec::new(),
        }
    }

    #[inline]
    pub fn set_parameter(&mut self, name: String, value: Float) {
        if let Some(param) = self.parameters.iter_mut().find(|(n, _)| n == &name) {
            param.1 = value;
        } else {
            self.parameters.push((name, value));
        }
    }

    #[inline]
    pub fn get_parameter(&self, name: &str) -> Option<Float> {
        self.parameters
            .iter()
            .find(|(n, _)| n == &name)
            .map(|(_, v)| *v)
    }

    #[inline]
    pub fn evaluate(&self) -> Vec<String> {
        // Simplified evaluation
        // In production, would return weighted blend of animation clips
        Vec::new()
    }
}
