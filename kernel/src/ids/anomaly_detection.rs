/// Anomaly Detection Module for IDS

extern crate alloc;
///
/// This module implements machine learning-based anomaly detection
/// to identify unusual patterns that may indicate security threats.

use crate::sync::{SpinLock, Mutex};
use crate::collections::VecDeque;
use crate::collections::HashMap;
use crate::compat::DefaultHasherBuilder;
use crate::time::{SystemTime, UNIX_EPOCH};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

/// Simple square root implementation for no_std environment
fn sqrt_f64(x: f64) -> f64 {
    if x < 0.0 {
        f64::NAN
    } else if x == 0.0 || x == 1.0 {
        x
    } else {
        // Newton's method
        let mut guess = x / 2.0;
        let mut prev = 0.0;
        while (guess - prev).abs() > 1e-10 {
            prev = guess;
            guess = (guess + x / guess) / 2.0;
        }
        guess
    }
}

/// Anomaly severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Anomaly category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyCategory {
    Network,
    Process,
    FileSystem,
    Memory,
    SystemCall,
    UserBehavior,
    Performance,
    Security,
}

/// Anomaly detection result
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Unique anomaly identifier
    pub id: u64,
    /// Anomaly category
    pub category: AnomalyCategory,
    /// Anomaly severity
    pub severity: AnomalySeverity,
    /// Anomaly description
    pub description: alloc::string::String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Anomaly timestamp
    pub timestamp: u64,
    /// Source of anomaly detection
    pub source: alloc::string::String,
    /// Relevant metrics or data points
    pub metrics: Vec<(alloc::string::String, f64)>,
    /// Context information
    pub context: HashMap<alloc::string::String, alloc::string::String>,
    /// Suggested actions
    pub suggested_actions: Vec<alloc::string::String>,
    /// Related entities
    pub related_entities: Vec<alloc::string::String>,
}

/// Statistical anomaly detection parameters
#[derive(Debug, Clone)]
pub struct StatisticalParams {
    /// Minimum data points for analysis
    pub min_data_points: usize,
    /// Standard deviation threshold
    pub std_dev_threshold: f64,
    /// Moving average window size
    pub moving_window: usize,
    /// Sensitivity level (0.0 - 1.0)
    pub sensitivity: f64,
}

impl Default for StatisticalParams {
    fn default() -> Self {
        Self {
            min_data_points: 30,
            std_dev_threshold: 2.0,
            moving_window: 10,
            sensitivity: 0.7,
        }
    }
}

/// Machine learning anomaly detection parameters
#[derive(Debug, Clone)]
pub struct MLParams {
    /// Number of clusters for unsupervised learning
    pub num_clusters: usize,
    /// Minimum samples for training
    pub min_training_samples: usize,
    /// Model update frequency (in samples)
    pub update_frequency: usize,
    /// Feature dimensionality
    pub feature_dimensions: usize,
    /// Learning rate for online learning
    pub learning_rate: f64,
}

impl Default for MLParams {
    fn default() -> Self {
        Self {
            num_clusters: 5,
            min_training_samples: 100,
            update_frequency: 50,
            feature_dimensions: 10,
            learning_rate: 0.01,
        }
    }
}

/// Anomaly detection algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionAlgorithm {
    Statistical,
    MachineLearning,
    Hybrid,
    Threshold,
    Pattern,
    Clustering,
}

/// Feature vector for anomaly detection
#[derive(Debug, Clone)]
pub struct FeatureVector {
    /// Feature values
    pub features: Vec<f64>,
    /// Feature labels
    pub labels: Vec<alloc::string::String>,
    /// Feature weights
    pub weights: Vec<f64>,
    /// Feature normalization
    pub normalized: bool,
}

impl FeatureVector {
    /// Create a new feature vector
    pub fn new() -> Self {
        Self {
            features: Vec::new(),
            labels: Vec::new(),
            weights: Vec::new(),
            normalized: false,
        }
    }

    /// Add a feature
    pub fn add_feature(&mut self, label: alloc::string::String, value: f64, weight: f64) {
        self.features.push(value);
        self.labels.push(label);
        self.weights.push(weight);
    }

    /// Normalize features (0-1 range)
    pub fn normalize(&mut self) {
        if self.features.is_empty() {
            return;
        }

        // Find min and max for each feature
        let mut min_vals = vec![f64::INFINITY; self.features.len()];
        let mut max_vals = vec![f64::NEG_INFINITY; self.features.len()];

        for i in 0..self.features.len() {
            if self.features[i] < min_vals[i] {
                min_vals[i] = self.features[i];
            }
            if self.features[i] > max_vals[i] {
                max_vals[i] = self.features[i];
            }
        }

        // Normalize each feature
        for i in 0..self.features.len() {
            if max_vals[i] - min_vals[i] > 0.0 {
                self.features[i] = (self.features[i] - min_vals[i]) / (max_vals[i] - min_vals[i]);
            }
        }

        self.normalized = true;
    }

    /// Calculate Euclidean distance to another feature vector
    pub fn distance_to(&self, other: &FeatureVector) -> f64 {
        if self.features.len() != other.features.len() {
            return f64::INFINITY;
        }

        let mut sum = 0.0;
        for i in 0..self.features.len() {
            let diff = self.features[i] - other.features[i];
            sum += diff * diff * self.weights[i] * other.weights[i];
        }

        sqrt_f64(sum)
    }
}

/// Statistical anomaly detector
pub struct StatisticalDetector {
    params: StatisticalParams,
    data_history: VecDeque<f64>,
    moving_average: f64,
    variance: f64,
    count: u64,
}

impl StatisticalDetector {
    /// Create a new statistical detector
    pub fn new(params: StatisticalParams) -> Self {
        Self {
            params,
            data_history: VecDeque::new(),
            moving_average: 0.0,
            variance: 0.0,
            count: 0,
        }
    }

    /// Add a new data point and check for anomaly
    pub fn analyze_point(&mut self, value: f64) -> Option<(bool, f32)> {
        self.count += 1;

        // Add to history
        if self.data_history.len() >= self.params.moving_window {
            self.data_history.pop_front();
        }
        self.data_history.push_back(value);

        // Update statistics
        if self.count == 1 {
            self.moving_average = value;
            self.variance = 0.0;
        } else {
            // Online update of mean and variance
            let alpha = 1.0 / self.count as f64;
            let delta = value - self.moving_average;
            self.moving_average += alpha * delta;
            self.variance = self.variance * (1.0 - alpha) + alpha * alpha * delta * delta * (self.count as f64 - 1.0);
        }

        // Check if we have enough data
        if self.count < self.params.min_data_points as u64 {
            return None;
        }

        // Calculate Z-score
        let std_dev = sqrt_f64(self.variance);
        if std_dev < f64::EPSILON {
            return None;
        }

        let z_score = (value - self.moving_average).abs() / std_dev;
        let anomaly_score = (z_score / self.params.std_dev_threshold) as f32;

        // Check if anomaly
        let is_anomaly = z_score > self.params.std_dev_threshold;

        if is_anomaly {
            Some((true, anomaly_score.min(1.0)))
        } else {
            Some((false, anomaly_score.min(1.0)))
        }
    }

    /// Get current statistics
    pub fn get_statistics(&self) -> (f64, f64, u64) {
        (self.moving_average, sqrt_f64(self.variance), self.count)
    }
}

/// Simple clustering anomaly detector
pub struct ClusteringDetector {
    params: MLParams,
    clusters: Vec<FeatureVector>,
    cluster_centers: Vec<FeatureVector>,
    cluster_sizes: Vec<usize>,
    trained: bool,
}

impl ClusteringDetector {
    /// Create a new clustering detector
    pub fn new(params: MLParams) -> Self {
        Self {
            params,
            clusters: Vec::new(),
            cluster_centers: Vec::new(),
            cluster_sizes: Vec::new(),
            trained: false,
        }
    }

    /// Train the detector with feature vectors
    pub fn train(&mut self, features: Vec<FeatureVector>) {
        if features.len() < self.params.min_training_samples {
            return;
        }

        // Initialize cluster centers using k-means++
        self.cluster_centers = self.initialize_centers(&features);

        // Run k-means clustering
        self.run_kmeans(&features);

        self.trained = true;
    }

    /// Analyze a feature vector for anomaly
    pub fn analyze(&self, feature: &FeatureVector) -> Option<(bool, f32)> {
        if !self.trained {
            return None;
        }

        // Find nearest cluster
        let mut min_distance = f64::INFINITY;
        let mut nearest_cluster = 0;

        for (i, center) in self.cluster_centers.iter().enumerate() {
            let distance = feature.distance_to(center);
            if distance < min_distance {
                min_distance = distance;
                nearest_cluster = i;
            }
        }

        // Calculate anomaly score based on distance and cluster size
        let cluster_density = self.cluster_sizes[nearest_cluster] as f64;
        let normalized_distance = min_distance / (sqrt_f64(cluster_density) + 1.0);
        let anomaly_score = (normalized_distance as f32).min(1.0);

        // Determine if anomaly (high distance from all clusters)
        let is_anomaly = anomaly_score > (1.0 - self.params.learning_rate as f32);

        Some((is_anomaly, anomaly_score))
    }

    /// Initialize cluster centers using k-means++
    fn initialize_centers(&self, features: &[FeatureVector]) -> Vec<FeatureVector> {
        let mut centers = Vec::new();

        if features.is_empty() {
            return centers;
        }

        // First center: random point
        let first_center = features[0].clone();
        centers.push(first_center);

        // Subsequent centers based on distance
        for _ in 1..self.params.num_clusters {
            let mut distances = Vec::new();

            for feature in features {
                let mut min_dist = f64::INFINITY;
                for center in &centers {
                    let dist = feature.distance_to(center);
                    if dist < min_dist {
                        min_dist = dist;
                    }
                }
                distances.push(min_dist * min_dist); // Square for probability
            }

            // Choose center with probability proportional to squared distance
            let total_dist: f64 = distances.iter().sum();
            if total_dist > 0.0 {
                let mut cumulative = 0.0;
                let random_val = total_dist * rand_f64();

                for (i, &dist) in distances.iter().enumerate() {
                    cumulative += dist;
                    if cumulative >= random_val {
                        centers.push(features[i].clone());
                        break;
                    }
                }
            }
        }

        centers
    }

    /// Run k-means clustering algorithm
    fn run_kmeans(&mut self, features: &[FeatureVector]) {
        let mut assignments = vec![0; features.len()];

        for _ in 0..20 { // Max iterations
            // Assign points to nearest cluster
            for (i, feature) in features.iter().enumerate() {
                let mut min_dist = f64::INFINITY;
                let mut cluster = 0;

                for (j, center) in self.cluster_centers.iter().enumerate() {
                    let dist = feature.distance_to(center);
                    if dist < min_dist {
                        min_dist = dist;
                        cluster = j;
                    }
                }

                assignments[i] = cluster;
            }

            // Update cluster centers
            let mut new_centers = vec![FeatureVector::new(); self.params.num_clusters];
            let mut new_sizes = vec![0; self.params.num_clusters];

            for (i, feature) in features.iter().enumerate() {
                let cluster = assignments[i];
                new_sizes[cluster] += 1;

                if new_centers[cluster].features.is_empty() {
                    new_centers[cluster] = feature.clone();
                } else {
                    // Simple averaging
                    for j in 0..feature.features.len() {
                        if j < new_centers[cluster].features.len() {
                            new_centers[cluster].features[j] += feature.features[j];
                        }
                    }
                }
            }

            // Normalize cluster centers
            for (i, center) in new_centers.iter_mut().enumerate() {
                if new_sizes[i] > 0 {
                    for feature in &mut center.features {
                        *feature /= new_sizes[i] as f64;
                    }
                }
            }

            self.cluster_centers = new_centers;
            self.cluster_sizes = new_sizes;
        }
    }
}

/// Main anomaly detection engine
pub struct AnomalyDetector {
    /// Detector configuration
    algorithm: DetectionAlgorithm,
    /// Statistical detector
    statistical: StatisticalDetector,
    /// Clustering detector
    clustering: ClusteringDetector,
    /// Detected anomalies
    anomalies: VecDeque<Anomaly>,
    /// Feature buffer for ML training
    feature_buffer: Vec<FeatureVector>,
    /// Anomaly counter
    anomaly_counter: AtomicU64,
    /// Is the detector trained
    trained: AtomicBool,
    /// Configuration lock
    config_lock: SpinLock,
}

impl AnomalyDetector {
    /// Create a new anomaly detector
    pub fn new(algorithm: DetectionAlgorithm) -> Self {
        Self {
            algorithm,
            statistical: StatisticalDetector::new(StatisticalParams::default()),
            clustering: ClusteringDetector::new(MLParams::default()),
            anomalies: VecDeque::new(),
            feature_buffer: Vec::new(),
            anomaly_counter: AtomicU64::new(0),
            trained: AtomicBool::new(false),
            config_lock: SpinLock::new(),
        }
    }

    /// Initialize the detector with configuration
    pub fn init(&mut self, config: &crate::ids::AnomalyDetectionConfig) -> Result<(), &'static str> {
        if !config.enabled {
            // If disabled, mark as untrained/idle
            self.trained.store(false, Ordering::SeqCst);
            return Ok(());
        }

        // Pick the first algorithm as the primary algorithm for simplicity
        if let Some(algo) = config.algorithms.first() {
            self.algorithm = match algo {
                crate::ids::AnomalyAlgorithm::Statistical => DetectionAlgorithm::Statistical,
                crate::ids::AnomalyAlgorithm::MachineLearning => DetectionAlgorithm::MachineLearning,
                _ => DetectionAlgorithm::Statistical,
            };
        }

        Ok(())
    }

    /// Gracefully shutdown the detector
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.trained.store(false, Ordering::SeqCst);
        self.anomalies.clear();
        self.feature_buffer.clear();
        Ok(())
    }

    /// Configure statistical parameters
    pub fn configure_statistical(&mut self, params: StatisticalParams) {
        let _lock = self.config_lock.lock();
        self.statistical = StatisticalDetector::new(params);
    }

    /// Configure ML parameters
    pub fn configure_ml(&mut self, params: MLParams) {
        let _lock = self.config_lock.lock();
        self.clustering = ClusteringDetector::new(params);
    }

    /// Analyze a single metric value
    pub fn analyze_metric(&mut self, metric_name: &str, value: f64) -> Option<Anomaly> {
        match self.algorithm {
            DetectionAlgorithm::Statistical => {
                if let Some((is_anomaly, score)) = self.statistical.analyze_point(value) {
                    if is_anomaly && score > 0.5 {
                        Some(self.create_anomaly(
                            AnomalyCategory::Performance,
                            AnomalySeverity::Medium,
                            alloc::format!("Statistical anomaly detected in metric: {}", metric_name),
                            score,
                            vec![(String::from(metric_name), value)],
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None, // Other algorithms need feature vectors
        }
    }

    /// Analyze feature vector for anomaly
    pub fn analyze_features(&mut self, feature: FeatureVector) -> Option<Anomaly> {
        let _lock = self.config_lock.lock();

        match self.algorithm {
            DetectionAlgorithm::MachineLearning | DetectionAlgorithm::Hybrid => {
                // Add to training buffer
                self.feature_buffer.push(feature.clone());

                // Train if we have enough samples
                if self.feature_buffer.len() >= self.clustering.params.min_training_samples &&
                   !self.trained.load(Ordering::Relaxed) {
                    self.clustering.train(self.feature_buffer.clone());
                    self.trained.store(true, Ordering::Relaxed);
                }

                // Analyze if trained
                if self.trained.load(Ordering::Relaxed) {
                    if let Some((is_anomaly, score)) = self.clustering.analyze(&feature) {
                        if is_anomaly && score > 0.5 {
                            return Some(self.create_anomaly(
                                AnomalyCategory::Security,
                                AnomalySeverity::High,
                                String::from("Machine learning anomaly detected"),
                                score,
                                feature.labels.iter().zip(feature.features.iter())
                                    .map(|(label, &value)| (label.clone(), value))
                                    .collect(),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }

        None
    }

    /// Get recent anomalies
    pub fn get_recent_anomalies(&self, count: usize) -> Vec<Anomaly> {
        self.anomalies.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get anomaly statistics
    pub fn get_statistics(&self) -> AnomalyStats {
        AnomalyStats {
            total_anomalies: self.anomaly_counter.load(Ordering::Relaxed),
            recent_anomalies: self.anomalies.len(),
            is_trained: self.trained.load(Ordering::Relaxed),
            detection_algorithm: self.algorithm,
        }
    }

    /// Clear old anomalies
    pub fn clear_old_anomalies(&mut self, max_age_seconds: u64) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        while let Some(anomaly) = self.anomalies.front() {
            if current_time - anomaly.timestamp > max_age_seconds {
                self.anomalies.pop_front();
            } else {
                break;
            }
        }
    }

    /// Create an anomaly object
    fn create_anomaly(
        &self,
        category: AnomalyCategory,
        severity: AnomalySeverity,
        description: alloc::string::String,
        confidence: f32,
        metrics: Vec<(alloc::string::String, f64)>,
    ) -> Anomaly {
        let id = self.anomaly_counter.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Anomaly {
            id,
            category,
            severity,
            description,
            confidence,
            timestamp,
            source: String::from("AnomalyDetector"),
            metrics,
            context: HashMap::with_hasher(DefaultHasherBuilder),
            suggested_actions: vec![
                String::from("Investigate the unusual pattern"),
                String::from("Check system logs for related events"),
            ],
            related_entities: Vec::new(),
        }
    }

    /// Add anomaly to storage
    fn add_anomaly(&mut self, anomaly: Anomaly) {
        if self.anomalies.len() >= 1000 {
            self.anomalies.pop_front();
        }
        self.anomalies.push_back(anomaly);
    }
}

/// Anomaly detection statistics
#[derive(Debug, Clone)]
pub struct AnomalyStats {
    /// Total anomalies detected
    pub total_anomalies: u64,
    /// Recent anomalies in memory
    pub recent_anomalies: usize,
    /// Is the detector trained
    pub is_trained: bool,
    /// Detection algorithm used
    pub detection_algorithm: DetectionAlgorithm,
}

/// Generate a simple random f64 value
fn rand_f64() -> f64 {
    // Simple pseudo-random generator
    static mut SEED: u64 = 12345;
    unsafe {
        SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
        (SEED as f64) / (u64::MAX as f64)
    }
}

/// Create a default anomaly detector
pub fn create_anomaly_detector() -> Arc<Mutex<AnomalyDetector>> {
    Arc::new(Mutex::new(AnomalyDetector::new(DetectionAlgorithm::Hybrid)))
}

/// Export anomaly data for external analysis
pub fn export_anomalies(anomalies: &[Anomaly]) -> alloc::string::String {
    let mut output = alloc::string::String::from("Anomaly Detection Report\n");
    output.push_str("========================\n\n");

    for anomaly in anomalies {
        output.push_str(&alloc::format!(
            "Anomaly ID: {}\nCategory: {:?}\nSeverity: {:?}\nConfidence: {:.2}\nDescription: {}\nTimestamp: {}\nMetrics: {}\nSuggested Actions: {}\n\n",
            anomaly.id,
            anomaly.category,
            anomaly.severity,
            anomaly.confidence,
            anomaly.description,
            anomaly.timestamp,
            anomaly.metrics.len(),
            anomaly.suggested_actions.join(", ")
        ));
    }

    output
}