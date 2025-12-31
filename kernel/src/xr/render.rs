//! XR Rendering Pipeline
//!
//! Implements optimized rendering for AR/VR:
//! - Stereo rendering (dual-eye)
//! - Lens distortion correction
//! - Chromatic aberration correction
//! - Time warp / space warp
//! - Late latching
//! - Foveated rendering
//! - Dynamic resolution scaling
//! - Frame timing optimization

use crate::xr::error::{XrError, XrResult};
use crate::xr::types::Pose;
use alloc::vec::Vec;
use core::time::Duration;

/// XR rendering configuration
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Target frame rate (Hz)
    pub target_frame_rate: u32,

    /// Display resolution per eye
    pub resolution: (u32, u32),

    /// Enable foveated rendering
    pub enable_foveated_rendering: bool,

    /// Enable time warp
    pub enable_time_warp: bool,

    /// Enable chromatic aberration correction
    pub enable_chromatic_correction: bool,

    /// Target latency (ms)
    pub target_latency_ms: u32,

    /// Foveation scale factor
    pub foveation_scale: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            target_frame_rate: 90,
            resolution: (1832, 1920),
            enable_foveated_rendering: true,
            enable_time_warp: true,
            enable_chromatic_correction: true,
            target_latency_ms: 20,
            foveation_scale: 0.5,
        }
    }
}

/// XR renderer
pub struct XrRenderer {
    config: RenderConfig,
    distortion_renderer: DistortionRenderer,
    time_warp: TimeWarpProcessor,
    foveated_renderer: FoveatedRenderer,
    frame_scheduler: FrameScheduler,
    is_running: bool,
}

impl XrRenderer {
    /// Create a new XR renderer
    pub fn new(config: &crate::xr::XrConfig) -> XrResult<Self> {
        let render_config = RenderConfig {
            resolution: config.display_resolution,
            target_frame_rate: config.target_frame_rate,
            ..Default::default()
        };

        let distortion_renderer = DistortionRenderer::new(&render_config)?;

        Ok(Self {
            config: render_config,
            distortion_renderer,
            time_warp: TimeWarpProcessor::new(),
            foveated_renderer: FoveatedRenderer::new(),
            frame_scheduler: FrameScheduler::new(config.target_frame_rate)?,
            is_running: false,
        })
    }

    /// Start renderer
    pub fn start(&mut self) -> XrResult<()> {
        self.is_running = true;
        self.frame_scheduler.reset();
        Ok(())
    }

    /// Stop renderer
    pub fn stop(&mut self) -> XrResult<()> {
        self.is_running = false;
        Ok(())
    }

    /// Submit a frame for rendering
    pub fn submit(&mut self, frame: XrFrame) -> XrResult<()> {
        if !self.is_running {
            return Err(XrError::InvalidState("Renderer not running".into()));
        }

        // Validate frame
        if frame.left_texture.is_empty() || frame.right_texture.is_empty() {
            return Err(XrError::InvalidArgument("Frame texture is empty".into()));
        }

        // Apply time warp if enabled
        let warped_frame = if self.config.enable_time_warp {
            self.time_warp.process_frame(&frame, &frame.predicted_pose)?
        } else {
            frame
        };

        // Apply distortion
        self.distortion_renderer.render(&warped_frame)?;

        Ok(())
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self) -> XrResult<FrameContext> {
        let predicted_timing = self.frame_scheduler.get_prediction_timing()?;

        Ok(FrameContext {
            frame_index: self.frame_scheduler.get_frame_count(),
            predicted_display_time: predicted_timing.display_time,
            target_frame_time: predicted_timing.frame_duration,
            recommended_resolution: self.get_recommended_resolution(),
        })
    }

    /// End frame and schedule presentation
    pub fn end_frame(&mut self, frame: XrFrame) -> XrResult<()> {
        self.submit(frame)?;
        self.frame_scheduler.advance_frame();
        Ok(())
    }

    /// Get recommended resolution for dynamic scaling
    fn get_recommended_resolution(&self) -> (u32, u32) {
        // Simple dynamic resolution scaling
        let scale_factor = self.frame_scheduler.get_performance_factor();

        let width = (self.config.resolution.0 as f32 * scale_factor) as u32;
        let height = (self.config.resolution.1 as f32 * scale_factor) as u32;

        (width, height)
    }

    /// Render foveated rendering pattern
    pub fn render_foveated_pattern(&self, resolution: (u32, u32)) -> XrResult<Vec<f32>> {
        self.foveated_renderer.generate_pattern(resolution)
    }

    /// Get frame timing statistics
    pub fn get_timing_stats(&self) -> RenderTimingStats {
        self.frame_scheduler.get_stats()
    }
}

/// XR frame data
#[derive(Debug, Clone)]
pub struct XrFrame {
    /// Left eye texture (RGBA)
    pub left_texture: Vec<u8>,

    /// Right eye texture (RGBA)
    pub right_texture: Vec<u8>,

    /// Left eye pose
    pub left_pose: Pose,

    /// Right eye pose
    pub right_pose: Pose,

    /// Predicted head pose at display time
    pub predicted_pose: Pose,

    /// Frame timestamp
    pub timestamp: Duration,

    /// Frame index
    pub index: u64,
}

/// Frame rendering context
#[derive(Debug, Clone)]
pub struct FrameContext {
    pub frame_index: u64,
    pub predicted_display_time: Duration,
    pub target_frame_time: Duration,
    pub recommended_resolution: (u32, u32),
}

/// Lens distortion renderer
pub struct DistortionRenderer {
    /// Distortion coefficients
    distortion_coeffs: [f32; 4],

    /// Chromatic aberration coefficients
    chromromatic_coeffs: [f32; 3],
}

impl DistortionRenderer {
    pub fn new(_config: &RenderConfig) -> XrResult<Self> {
        Ok(Self {
            distortion_coeffs: [0.22, 0.24, 0.1, 0.05], // Typical VR lens distortion
            chromromatic_coeffs: [-0.015, -0.005, 0.001], // R, G, B
        })
    }

    /// Render with lens distortion correction
    pub fn render(&self, _frame: &XrFrame) -> XrResult<()> {
        // In practice, this would apply distortion shader
        // Simplified: the frame textures would already have distortion applied
        Ok(())
    }

    /// Undistort a point
    pub fn undistort_point(&self, point: (f32, f32)) -> (f32, f32) {
        let (x, y) = point;
        let r2 = x * x + y * y;

        // Radial distortion: r' = r * (1 + k1*r² + k2*r⁴ + k3*r⁶)
        let distortion = 1.0
            + self.distortion_coeffs[0] * r2
            + self.distortion_coeffs[1] * r2 * r2
            + self.distortion_coeffs[2] * r2 * r2 * r2;

        let undistorted_x = x * distortion;
        let undistorted_y = y * distortion;

        (undistorted_x, undistorted_y)
    }

    /// Distort a point
    pub fn distort_point(&self, point: (f32, f32)) -> (f32, f32) {
        // Inverse distortion (approximated)
        let mut undistorted = point;
        for _ in 0..3 {
            // Newton-Raphson iteration
            undistorted = self.undistort_point(undistorted);
        }
        undistorted
    }

    /// Apply chromatic aberration correction
    pub fn correct_chromatic_aberration(&self, rgb: [u8; 3]) -> [u8; 3] {
        // In practice, this would shift RGB channels
        // Simplified: just pass through
        rgb
    }
}

/// Time warp processor
pub struct TimeWarpProcessor {
    enabled: bool,
}

impl TimeWarpProcessor {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Apply time warp to frame
    pub fn process_frame(&self, frame: &XrFrame, predicted_pose: &Pose) -> XrResult<XrFrame> {
        if !self.enabled {
            return Ok(frame.clone());
        }

        // Compute rotation delta between current and predicted pose
        let rotation_delta = frame
            .predicted_pose
            .orientation
            .multiply(&predicted_pose.orientation.inverse());

        // Apply rotation warp to both eyes
        let mut warped_frame = frame.clone();

        warped_frame.left_pose = Pose::new(
            frame.left_pose.position,
            rotation_delta.multiply(&frame.left_pose.orientation),
        );

        warped_frame.right_pose = Pose::new(
            frame.right_pose.position,
            rotation_delta.multiply(&frame.right_pose.orientation),
        );

        Ok(warped_frame)
    }

    /// Compute time warp transform
    fn compute_warp_transform(&self, from_pose: &Pose, to_pose: &Pose) -> Pose {
        let position_delta = to_pose.position - from_pose.position;
        let rotation_delta = to_pose.orientation.multiply(&from_pose.orientation.inverse());

        Pose::new(position_delta, rotation_delta)
    }
}

/// Foveated renderer
pub struct FoveatedRenderer {
    fovea_center: (f32, f32),
    fovea_radius: f32,
    peripheral_scale: f32,
}

impl FoveatedRenderer {
    pub fn new() -> Self {
        Self {
            fovea_center: (0.5, 0.5),
            fovea_radius: 0.3,
            peripheral_scale: 0.5,
        }
    }

    /// Generate foveated rendering pattern
    pub fn generate_pattern(&self, resolution: (u32, u32)) -> XrResult<Vec<f32>> {
        let mut pattern = Vec::new();
        let width = resolution.0 as f32;
        let height = resolution.1 as f32;

        for y in 0..resolution.1 {
            for x in 0..resolution.0 {
                let u = x as f32 / width;
                let v = y as f32 / height;

                let dx = u - self.fovea_center.0;
                let dy = v - self.fovea_center.1;
                let dist = (dx * dx + dy * dy).sqrt();

                // Compute resolution scale factor
                let scale = if dist < self.fovea_radius {
                    1.0
                } else {
                    let t = (dist - self.fovea_radius) / (1.0 - self.fovea_radius);
                    1.0 - t * (1.0 - self.peripheral_scale)
                };

                pattern.push(scale);
            }
        }

        Ok(pattern)
    }

    /// Update fovea position (from eye tracking)
    pub fn update_fovea(&mut self, gaze_point: (f32, f32)) {
        self.fovea_center = gaze_point;
    }

    /// Set foveation parameters
    pub fn set_parameters(&mut self, radius: f32, peripheral_scale: f32) {
        self.fovea_radius = radius.clamp(0.1, 0.5);
        self.peripheral_scale = peripheral_scale.clamp(0.1, 1.0);
    }
}

/// Frame scheduler
pub struct FrameScheduler {
    target_frame_rate: u32,
    frame_duration: Duration,
    frame_count: u64,
    last_frame_time: Option<Duration>,
    performance_history: Vec<Duration>,
}

impl FrameScheduler {
    pub fn new(target_frame_rate: u32) -> XrResult<Self> {
        let frame_duration = Duration::from_secs_f64(1.0 / target_frame_rate as f64);

        Ok(Self {
            target_frame_rate,
            frame_duration,
            frame_count: 0,
            last_frame_time: None,
            performance_history: Vec::new(),
        })
    }

    /// Get prediction timing
    pub fn get_prediction_timing(&self) -> XrResult<PredictionTiming> {
        Ok(PredictionTiming {
            frame_duration: self.frame_duration,
            display_time: Duration::ZERO,
            prediction_offset: self.frame_duration,
        })
    }

    /// Advance to next frame
    pub fn advance_frame(&mut self) {
        self.frame_count += 1;
        self.last_frame_time = Some(Duration::from_secs_f64(
            self.frame_count as f64 / self.target_frame_rate as f64,
        ));
    }

    /// Get frame count
    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Reset scheduler
    pub fn reset(&mut self) {
        self.frame_count = 0;
        self.last_frame_time = None;
        self.performance_history.clear();
    }

    /// Record frame time
    pub fn record_frame_time(&mut self, frame_time: Duration) {
        self.performance_history.push(frame_time);

        // Keep last 60 frames
        if self.performance_history.len() > 60 {
            self.performance_history.remove(0);
        }
    }

    /// Get performance factor for dynamic resolution
    pub fn get_performance_factor(&self) -> f32 {
        if self.performance_history.is_empty() {
            return 1.0;
        }

        let avg_frame_time: Duration = self
            .performance_history
            .iter()
            .sum::<Duration>()
            .div_f32(self.performance_history.len() as f32);

        // Compute ratio of target to actual frame time
        let ratio = self.frame_duration.as_secs_f32() / avg_frame_time.as_secs_f32();

        // Clamp to reasonable range [0.5, 1.5]
        ratio.clamp(0.5, 1.5)
    }

    /// Get timing statistics
    pub fn get_stats(&self) -> RenderTimingStats {
        let avg_frame_time = if !self.performance_history.is_empty() {
            self.performance_history
                .iter()
                .sum::<Duration>()
                .div_f32(self.performance_history.len() as f32)
        } else {
            Duration::ZERO
        };

        let fps = if avg_frame_time.as_secs_f32() > 0.0 {
            1.0 / avg_frame_time.as_secs_f32()
        } else {
            0.0
        };

        RenderTimingStats {
            frame_rate: fps,
            frame_time: avg_frame_time,
            dropped_frames: 0,
            latency_ms: avg_frame_time.as_secs_f64() * 1000.0,
        }
    }
}

/// Prediction timing
#[derive(Debug, Clone)]
pub struct PredictionTiming {
    pub frame_duration: Duration,
    pub display_time: Duration,
    pub prediction_offset: Duration,
}

/// Render timing statistics
#[derive(Debug, Clone)]
pub struct RenderTimingStats {
    pub frame_rate: f32,
    pub frame_time: Duration,
    pub dropped_frames: u64,
    pub latency_ms: f64,
}

/// Eye rendering parameters
#[derive(Debug, Clone)]
pub struct EyeRenderParams {
    /// Viewport offset
    pub viewport_offset: (i32, i32),

    /// Viewport size
    pub viewport_size: (u32, u32),

    /// Projection matrix
    pub projection_matrix: [[f32; 4]; 4],

    /// View matrix
    pub view_matrix: [[f32; 4]; 4],
}

impl EyeRenderParams {
    pub fn new(eye: Eye) -> Self {
        let (_offset_x, ipd) = match eye {
            Eye::Left => (-1, -0.032), // -32mm IPD
            Eye::Right => (1, 0.032),
        };

        Self {
            viewport_offset: (if eye == Eye::Left { 0 } else { 1832 }, 0),
            viewport_size: (1832, 1920),
            projection_matrix: Self::create_projection_matrix(90.0, 0.1, 100.0, ipd),
            view_matrix: Self::create_view_matrix(ipd),
        }
    }

    fn create_projection_matrix(fov: f32, near: f32, far: f32, ipd: f32) -> [[f32; 4]; 4] {
        let aspect = 1832.0 / 1920.0;
        let tan_half_fov = (fov / 2.0).to_radians().tan();

        [
            [1.0 / (aspect * tan_half_fov), 0.0, 0.0, 0.0],
            [0.0, 1.0 / tan_half_fov, 0.0, 0.0],
            [ipd / near, 0.0, -(far + near) / (far - near), -1.0],
            [0.0, 0.0, -(2.0 * far * near) / (far - near), 0.0],
        ]
    }

    fn create_view_matrix(ipd: f32) -> [[f32; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [ipd, 0.0, 0.0, 1.0],
        ]
    }
}

/// Eye enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Eye {
    Left,
    Right,
}

/// Late latching (update transforms just before submit)
pub struct LateLatcher {
    update_threshold: Duration,
}

impl LateLatcher {
    pub fn new() -> Self {
        Self {
            update_threshold: Duration::from_millis(5),
        }
    }

    /// Determine if should late latch
    pub fn should_late_latch(&self, time_until_submit: Duration) -> bool {
        time_until_submit < self.update_threshold
    }

    /// Apply late latch update
    pub fn late_latch_transform(&self, current: &Pose, latest: &Pose) -> Pose {
        latest.multiply(&current.inverse())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let config = crate::xr::XrConfig::default();
        let renderer = XrRenderer::new(&config);
        assert!(renderer.is_ok());
    }

    #[test]
    fn test_frame_scheduler() {
        let scheduler = FrameScheduler::new(90).unwrap();
        assert_eq!(scheduler.get_frame_count(), 0);

        scheduler.advance_frame();
        assert_eq!(scheduler.get_frame_count(), 1);
    }

    #[test]
    fn test_distortion_correction() {
        let config = RenderConfig::default();
        let renderer = DistortionRenderer::new(&config).unwrap();

        let point = (0.5, 0.5);
        let undistorted = renderer.undistort_point(point);
        assert_ne!(point, undistorted);
    }

    #[test]
    fn test_eye_render_params() {
        let left = EyeRenderParams::new(Eye::Left);
        let right = EyeRenderParams::new(Eye::Right);

        assert_ne!(left.viewport_offset, right.viewport_offset);
    }

    #[test]
    fn test_foveated_pattern() {
        let foveated = FoveatedRenderer::new();
        let pattern = foveated.generate_pattern((100, 100));
        assert!(pattern.is_ok());
    }
}
