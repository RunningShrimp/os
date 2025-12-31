//! Hand Tracking and Gesture Recognition
//!
//! Implements real-time hand tracking and gesture recognition:
//! - Hand detection and localization
//! - Hand pose estimation (21 keypoints)
//! - Gesture recognition (pinch, grab, point, etc.)
//! - Fingertip tracking
//! - Grab detection
//! - Gesture-based commands

use crate::xr::error::{XrError, XrResult};
use crate::xr::types::{Pose, Vector3, Quaternion};
use alloc::vec::Vec;

/// Hand tracking configuration
#[derive(Debug, Clone)]
pub struct HandTrackingConfig {
    /// Enable left hand tracking
    pub enable_left_hand: bool,

    /// Enable right hand tracking
    pub enable_right_hand: bool,

    /// Gesture detection confidence threshold
    pub gesture_confidence_threshold: f32,

    /// Pinch distance threshold (meters)
    pub pinch_threshold: f32,

    /// Grab activation threshold
    pub grab_threshold: f32,

    /// Maximum hand detection distance (meters)
    pub max_detection_distance: f32,
}

impl Default for HandTrackingConfig {
    fn default() -> Self {
        Self {
            enable_left_hand: true,
            enable_right_hand: true,
            gesture_confidence_threshold: 0.7,
            pinch_threshold: 0.05,
            grab_threshold: 0.1,
            max_detection_distance: 1.0,
        }
    }
}

/// Hand tracker
pub struct HandTracker {
    config: HandTrackingConfig,
    left_hand: Option<HandTrackingData>,
    right_hand: Option<HandTrackingData>,
    gesture_recognizer: GestureRecognizer,
    is_running: bool,
}

impl HandTracker {
    /// Create a new hand tracker
    pub fn new() -> XrResult<Self> {
        Ok(Self {
            config: HandTrackingConfig::default(),
            left_hand: None,
            right_hand: None,
            gesture_recognizer: GestureRecognizer::new(),
            is_running: false,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: HandTrackingConfig) -> XrResult<Self> {
        Ok(Self {
            config,
            left_hand: None,
            right_hand: None,
            gesture_recognizer: GestureRecognizer::new(),
            is_running: false,
        })
    }

    /// Start hand tracking
    pub fn start(&mut self) -> XrResult<()> {
        self.is_running = true;
        Ok(())
    }

    /// Stop hand tracking
    pub fn stop(&mut self) -> XrResult<()> {
        self.is_running = false;
        self.left_hand = None;
        self.right_hand = None;
        Ok(())
    }

    /// Process camera image for hand detection
    pub fn process_image(&mut self, image_data: &[u8], timestamp: u64) -> XrResult<Vec<HandPose>> {
        if !self.is_running {
            return Err(XrError::InvalidState("Hand tracker not running".into()));
        }

        // Detect hands in image
        let detected_hands = self.detect_hands(image_data)?;

        // Update tracking data
        for detected in detected_hands {
            match detected.handedness {
                Handedness::Left if self.config.enable_left_hand => {
                    self.left_hand = Some(HandTrackingData {
                        pose: detected.pose,
                        confidence: detected.confidence,
                        last_seen: timestamp,
                    });
                }
                Handedness::Right if self.config.enable_right_hand => {
                    self.right_hand = Some(HandTrackingData {
                        pose: detected.pose,
                        confidence: detected.confidence,
                        last_seen: timestamp,
                    });
                }
                _ => {}
            }
        }

        // Get current hand poses
        self.get_poses()
    }

    /// Get current hand poses
    pub fn get_poses(&self) -> XrResult<Vec<HandPose>> {
        let mut poses = Vec::new();

        if let Some(left) = &self.left_hand {
            let gesture = self.gesture_recognizer.recognize(&left.pose.keypoints);
            poses.push(HandPose {
                handedness: Handedness::Left,
                pose: Pose::new(left.pose.position, left.pose.orientation),
                keypoints: left.pose.keypoints.clone(),
                gesture,
                confidence: left.confidence,
                is_visible: true,
            });
        }

        if let Some(right) = &self.right_hand {
            let gesture = self.gesture_recognizer.recognize(&right.pose.keypoints);
            poses.push(HandPose {
                handedness: Handedness::Right,
                pose: Pose::new(right.pose.position, right.pose.orientation),
                keypoints: right.pose.keypoints.clone(),
                gesture,
                confidence: right.confidence,
                is_visible: true,
            });
        }

        Ok(poses)
    }

    /// Get pinch strength (0-1)
    pub fn get_pinch_strength(&self, handedness: Handedness) -> XrResult<f32> {
        let hand_data = match handedness {
            Handedness::Left => &self.left_hand,
            Handedness::Right => &self.right_hand,
        };

        match hand_data {
            Some(data) => {
                let thumb_tip = &data.pose.keypoints[4]; // Thumb tip
                let index_tip = &data.pose.keypoints[8]; // Index tip

                let distance = thumb_tip.position.distance_to(&index_tip.position);
                let strength = 1.0 - (distance / self.config.pinch_threshold).min(1.0);
                Ok(strength)
            }
            None => Ok(0.0),
        }
    }

    /// Detect hands in image
    fn detect_hands(&self, _image_data: &[u8]) -> XrResult<Vec<DetectedHand>> {
        let mut detected = Vec::new();

        // Simplified hand detection
        // In practice, this would use a CNN-based detector (MediaPipe, etc.)
        if self.config.enable_left_hand {
            detected.push(DetectedHand {
                handedness: Handedness::Left,
                pose: HandSkeleton::default(),
                confidence: 0.8,
            });
        }

        if self.config.enable_right_hand {
            detected.push(DetectedHand {
                handedness: Handedness::Right,
                pose: HandSkeleton::default(),
                confidence: 0.8,
            });
        }

        Ok(detected)
    }

    /// Check if hand is pinching
    pub fn is_pinching(&self, handedness: Handedness) -> bool {
        match self.get_pinch_strength(handedness) {
            Ok(strength) => strength > 0.8,
            Err(_) => false,
        }
    }

    /// Check if hand is grabbing
    pub fn is_grabbing(&self, handedness: Handedness) -> bool {
        let hand_data = match handedness {
            Handedness::Left => &self.left_hand,
            Handedness::Right => &self.right_hand,
        };

        match hand_data {
            Some(data) => {
                // Check if fingers are curled
                let palm_center = &data.pose.keypoints[0];
                let finger_tips = [&data.pose.keypoints[8], &data.pose.keypoints[12], &data.pose.keypoints[16], &data.pose.keypoints[20]];

                let mut curled_count = 0;
                for tip in finger_tips {
                    let distance = palm_center.position.distance_to(&tip.position);
                    if distance < self.config.grab_threshold {
                        curled_count += 1;
                    }
                }

                curled_count >= 3
            }
            None => false,
        }
    }
}

/// Hand tracking data
#[derive(Debug, Clone)]
struct HandTrackingData {
    pose: HandSkeleton,
    confidence: f32,
    last_seen: u64,
}

/// Detected hand
#[derive(Debug, Clone)]
struct DetectedHand {
    handedness: Handedness,
    pose: HandSkeleton,
    confidence: f32,
}

/// Hand pose with full skeleton
#[derive(Debug, Clone)]
pub struct HandPose {
    pub handedness: Handedness,
    pub pose: Pose,
    pub keypoints: Vec<Keypoint>,
    pub gesture: Gesture,
    pub confidence: f32,
    pub is_visible: bool,
}

/// Hand skeleton with 21 keypoints
#[derive(Debug, Clone)]
pub struct HandSkeleton {
    pub position: Vector3<f32>,
    pub orientation: Quaternion,
    pub keypoints: Vec<Keypoint>,
}

impl Default for HandSkeleton {
    fn default() -> Self {
        // Create default hand skeleton
        let mut keypoints = Vec::new();
        for i in 0..21 {
            keypoints.push(Keypoint {
                id: i,
                position: Vector3::new(0.0, 0.0, 0.0),
                confidence: 1.0,
            });
        }

        Self {
            position: Vector3::new(0.3, -0.3, -0.5),
            orientation: Quaternion::identity(),
            keypoints,
        }
    }
}

/// Hand keypoint (joint)
#[derive(Debug, Clone)]
pub struct Keypoint {
    pub id: usize,
    pub position: Vector3<f32>,
    pub confidence: f32,
}

/// Left or right hand
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Handedness {
    Left,
    Right,
}

/// Recognized gesture
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    /// No specific gesture
    None,

    /// Pointing with index finger
    Point,

    /// Pinching thumb and index finger
    Pinch,

    /// Grabbing/closing hand
    Grab,

    /// Open palm
    OpenPalm,

    /// Thumbs up
    ThumbsUp,

    /// Thumbs down
    ThumbsDown,

    /// Victory sign (two fingers)
    Victory,

    /// OK sign
    Ok,

    /// Fist
    Fist,

    /// Custom gesture
    Custom(u32),
}

/// Gesture recognizer
pub struct GestureRecognizer {
    gesture_history: Vec<(Gesture, u64)>,
}

impl GestureRecognizer {
    pub fn new() -> Self {
        Self {
            gesture_history: Vec::new(),
        }
    }

    /// Recognize gesture from hand keypoints
    pub fn recognize(&self, keypoints: &[Keypoint]) -> Gesture {
        if keypoints.len() < 21 {
            return Gesture::None;
        }

        // Get key joints
        let wrist = &keypoints[0];
        let thumb_tip = &keypoints[4];
        let index_tip = &keypoints[8];
        let middle_tip = &keypoints[12];
        let ring_tip = &keypoints[16];
        let pinky_tip = &keypoints[20];

        let index_mcp = &keypoints[5];
        let middle_mcp = &keypoints[9];
        let ring_mcp = &keypoints[13];
        let pinky_mcp = &keypoints[17];

        // Check for pointing
        let index_extended = index_tip.position.distance_to(&wrist.position)
            > index_mcp.position.distance_to(&wrist.position) * 2.0;

        let middle_curled = middle_tip.position.distance_to(&wrist.position)
            < middle_mcp.position.distance_to(&wrist.position) * 1.2;

        let ring_curled = ring_tip.position.distance_to(&wrist.position)
            < ring_mcp.position.distance_to(&wrist.position) * 1.2;

        let pinky_curled = pinky_tip.position.distance_to(&wrist.position)
            < pinky_mcp.position.distance_to(&wrist.position) * 1.2;

        if index_extended && middle_curled && ring_curled && pinky_curled {
            return Gesture::Point;
        }

        // Check for pinch
        let pinch_distance = thumb_tip.position.distance_to(&index_tip.position);
        if pinch_distance < 0.03 {
            return Gesture::Pinch;
        }

        // Check for open palm
        let all_extended = index_extended &&
            middle_tip.position.distance_to(&wrist.position) > middle_mcp.position.distance_to(&wrist.position) * 1.5 &&
            ring_tip.position.distance_to(&wrist.position) > ring_mcp.position.distance_to(&wrist.position) * 1.5 &&
            pinky_tip.position.distance_to(&wrist.position) > pinky_mcp.position.distance_to(&wrist.position) * 1.5;

        if all_extended {
            return Gesture::OpenPalm;
        }

        // Check for fist
        let all_curled = !index_extended && middle_curled && ring_curled && pinky_curled;
        if all_curled {
            return Gesture::Fist;
        }

        // Check for victory
        let index_extended = index_tip.position.distance_to(&wrist.position)
            > index_mcp.position.distance_to(&wrist.position) * 1.5;

        let middle_extended = middle_tip.position.distance_to(&wrist.position)
            > middle_mcp.position.distance_to(&wrist.position) * 1.5;

        if index_extended && middle_extended && ring_curled && pinky_curled {
            return Gesture::Victory;
        }

        Gesture::None
    }

    /// Get gesture stability (how consistent the gesture is)
    pub fn get_stability(&self) -> f32 {
        if self.gesture_history.len() < 5 {
            return 0.0;
        }

        let recent = &self.gesture_history[self.gesture_history.len() - 5..];
        let first_gesture = recent[0].0;

        let consistent = recent.iter().filter(|(g, _)| *g == first_gesture).count();
        consistent as f32 / recent.len() as f32
    }

    /// Update gesture history
    pub fn update_history(&mut self, gesture: Gesture, timestamp: u64) {
        self.gesture_history.push((gesture, timestamp));

        // Keep only recent history
        if self.gesture_history.len() > 100 {
            self.gesture_history.remove(0);
        }
    }
}

/// Fingertip tracker for precise interaction
pub struct FingertipTracker {
    smoothing_window: usize,
    tip_positions: Vec<Vector3<f32>>,
}

impl FingertipTracker {
    pub fn new() -> Self {
        Self {
            smoothing_window: 5,
            tip_positions: Vec::new(),
        }
    }

    /// Track fingertip position
    pub fn track(&mut self, position: Vector3<f32>) -> Vector3<f32> {
        self.tip_positions.push(position);

        if self.tip_positions.len() > self.smoothing_window {
            self.tip_positions.remove(0);
        }

        self.get_smoothed_position()
    }

    fn get_smoothed_position(&self) -> Vector3<f32> {
        if self.tip_positions.is_empty() {
            return Vector3::ZERO;
        }

        let mut sum = Vector3::ZERO;
        for pos in &self.tip_positions {
            sum = sum + *pos;
        }

        let count = self.tip_positions.len() as f32;
        Vector3::new(sum.x / count, sum.y / count, sum.z / count)
    }

    /// Reset tracker
    pub fn reset(&mut self) {
        self.tip_positions.clear();
    }
}

/// Grab detector
pub struct GrabDetector {
    is_grabbing: bool,
    grab_start_time: Option<u64>,
    grab_activation_time: u64,
}

impl GrabDetector {
    pub fn new() -> Self {
        Self {
            is_grabbing: false,
            grab_start_time: None,
            grab_activation_time: 150, // ms
        }
    }

    /// Update grab detection
    pub fn update(&mut self, is_fingers_curled: bool, timestamp: u64) -> GrabState {
        if is_fingers_curled && !self.is_grabbing {
            // Start grab debounce
            if self.grab_start_time.is_none() {
                self.grab_start_time = Some(timestamp);
            }

            let elapsed = timestamp - self.grab_start_time.unwrap();
            if elapsed > self.grab_activation_time {
                self.is_grabbing = true;
                return GrabState::Started;
            }

            GrabState::Starting
        } else if !is_fingers_curled && self.is_grabbing {
            // Release grab
            self.is_grabbing = false;
            self.grab_start_time = None;
            GrabState::Released
        } else if self.is_grabbing {
            GrabState::Holding
        } else {
            self.grab_start_time = None;
            GrabState::None
        }
    }

    /// Reset detector
    pub fn reset(&mut self) {
        self.is_grabbing = false;
        self.grab_start_time = None;
    }
}

/// Grab state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GrabState {
    /// Not grabbing
    None,

    /// Grab is starting (debounce period)
    Starting,

    /// Grab has started
    Started,

    /// Currently grabbing
    Holding,

    /// Grab has been released
    Released,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hand_tracker_creation() {
        let tracker = HandTracker::new();
        assert!(tracker.is_ok());
    }

    #[test]
    fn test_gesture_recognition() {
        let recognizer = GestureRecognizer::new();
        let keypoints = vec![Keypoint {
            id: 0,
            position: Vector3::new(0.0, 0.0, 0.0),
            confidence: 1.0,
        }; 21];

        let gesture = recognizer.recognize(&keypoints);
        assert!(matches!(gesture, Gesture::None));
    }

    #[test]
    fn test_fingertip_tracker() {
        let mut tracker = FingertipTracker::new();
        let pos = Vector3::new(1.0, 2.0, 3.0);
        let tracked = tracker.track(pos);
        assert_eq!(tracked, pos);
    }

    #[test]
    fn test_grab_detector() {
        let mut detector = GrabDetector::new();
        let state = detector.update(false, 1000);
        assert_eq!(state, GrabState::None);
    }
}
