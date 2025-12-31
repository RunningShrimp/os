//! # Animation State Machine
//!
//! State machine for managing animation transitions.

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use crate::compat::Float;
use super::AnimationClip;

/// Animation state
#[derive(Clone, Debug)]
pub struct AnimationStateDefinition {
    pub name: String,
    pub clip: AnimationClip,
    pub speed: Float,
    pub looping: bool,
}

impl AnimationStateDefinition {
    #[inline]
    pub fn new(name: String, clip: AnimationClip) -> Self {
        Self {
            name,
            clip,
            speed: 1.0,
            looping: true,
        }
    }
}

/// Transition between states
#[derive(Clone, Debug)]
pub struct StateTransition {
    pub target_state: String,
    pub duration: Float,
    pub exit_time: Float,
    pub conditions: Vec<TransitionCondition>,
}

impl StateTransition {
    #[inline]
    pub fn new(target_state: String) -> Self {
        Self {
            target_state,
            duration: 0.3,
            exit_time: 0.0,
            conditions: Vec::new(),
        }
    }
}

/// Transition condition
#[derive(Clone, Debug)]
pub enum TransitionCondition {
    TimeElapsed(Float),
    Parameter(String, f32),
    Trigger(String),
    Boolean(String, bool),
}

/// Animation state machine
#[derive(Clone, Debug)]
pub struct AnimationStateMachine {
    pub states: BTreeMap<String, AnimationStateDefinition>,
    pub transitions: BTreeMap<String, Vec<StateTransition>>,
    pub current_state: Option<String>,
    pub state_time: Float,
    pub parameters: BTreeMap<String, AnimationParameter>,
}

impl AnimationStateMachine {
    #[inline]
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            transitions: BTreeMap::new(),
            current_state: None,
            state_time: 0.0,
            parameters: BTreeMap::new(),
        }
    }

    #[inline]
    pub fn add_state(&mut self, state: AnimationStateDefinition) {
        let name = state.name.clone();
        self.states.insert(name, state);
    }

    #[inline]
    pub fn add_transition(&mut self, from_state: String, transition: StateTransition) {
        self.transitions
            .entry(from_state)
            .or_insert_with(Vec::new)
            .push(transition);
    }

    #[inline]
    pub fn set_state(&mut self, state_name: &str) {
        if self.states.contains_key(state_name) {
            self.current_state = Some(state_name.to_string());
            self.state_time = 0.0;
        }
    }

    #[inline]
    pub fn update(&mut self, dt: Float) -> Option<String> {
        if let Some(current) = &self.current_state {
            self.state_time += dt;

            // Check transitions
            if let Some(transitions) = self.transitions.get(current) {
                for transition in transitions {
                    if self.check_transition(transition) {
                        return Some(transition.target_state.clone());
                    }
                }
            }
        }

        None
    }

    #[inline]
    fn check_transition(&self, transition: &StateTransition) -> bool {
        if transition.exit_time > 0.0 && self.state_time < transition.exit_time {
            return false;
        }

        for condition in &transition.conditions {
            if !self.check_condition(condition) {
                return false;
            }
        }

        true
    }

    #[inline]
    fn check_condition(&self, condition: &TransitionCondition) -> bool {
        match condition {
            TransitionCondition::TimeElapsed(time) => self.state_time >= *time,
            TransitionCondition::Parameter(name, value) => {
                if let Some(param) = self.parameters.get(name) {
                    match param {
                        AnimationParameter::Float(v) => (v - value).abs() < 0.001,
                        _ => false,
                    }
                } else {
                    false
                }
            }
            TransitionCondition::Trigger(name) => {
                if let Some(AnimationParameter::Trigger(triggered)) = self.parameters.get(name) {
                    *triggered
                } else {
                    false
                }
            }
            TransitionCondition::Boolean(name, value) => {
                if let Some(AnimationParameter::Bool(v)) = self.parameters.get(name) {
                    v == value
                } else {
                    false
                }
            }
        }
    }

    #[inline]
    pub fn set_parameter(&mut self, name: String, parameter: AnimationParameter) {
        self.parameters.insert(name, parameter);
    }
}

impl Default for AnimationStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// Animation parameter
#[derive(Clone, Debug)]
pub enum AnimationParameter {
    Float(f32),
    Int(i32),
    Bool(bool),
    Trigger(bool),
}
