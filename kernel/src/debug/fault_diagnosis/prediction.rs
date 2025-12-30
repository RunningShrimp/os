//! Prediction models

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// 预测模型
#[derive(Debug, Clone)]
pub struct PredictionModel {
    /// 模型ID
    pub id: String,
    /// 模型名称
    pub name: String,
    /// 模型类型
    pub model_type: PredictionModelType,
    /// 输入特征
    pub input_features: Vec<String>,
    /// 输出预测
    pub output_predictions: Vec<String>,
    /// 模型参数
    pub model_parameters: BTreeMap<String, String>,
    /// 训练数据集
    pub training_dataset: String,
    /// 模型准确率
    pub accuracy: f64,
    /// 最后训练时间
    pub last_trained: u64,
    /// 预测窗口（小时）
    pub prediction_window_hours: u64,
}

/// 预测模型类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionModelType {
    /// 时间序列预测
    TimeSeries,
    /// 分类预测
    Classification,
    /// 回归预测
    Regression,
    /// 异常预测
    AnomalyPrediction,
    /// 生存分析
    SurvivalAnalysis,
}
