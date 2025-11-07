//! GPU monitoring module
//!
//! Provides GPU metrics with platform-specific detection

use serde::{Deserialize, Serialize};

/// GPU metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub gpu_name: String,
    pub provider: String, // "cuda", "cpu", "directml", "coreml"
    pub utilization_percent: Option<f32>,
    pub memory_used_mb: Option<u64>,
    pub memory_total_mb: Option<u64>,
    pub temperature_c: Option<f32>,
}

impl Default for GpuMetrics {
    fn default() -> Self {
        Self {
            gpu_name: "Unknown".to_string(),
            provider: "cpu".to_string(),
            utilization_percent: None,
            memory_used_mb: None,
            memory_total_mb: None,
            temperature_c: None,
        }
    }
}

/// GPU monitoring interface
pub struct GpuMonitor {
    provider: String,
    gpu_name: String,
}

impl GpuMonitor {
    /// Create new GPU monitor for given provider
    pub fn new(provider: &str) -> Self {
        let gpu_name = match provider {
            "cuda" => "NVIDIA GPU (CUDA)".to_string(),
            "directml" => "DirectML GPU".to_string(),
            "coreml" => "Apple Silicon (CoreML)".to_string(),
            "cpu" => "CPU Fallback".to_string(),
            _ => format!("Unknown ({})", provider),
        };

        Self {
            provider: provider.to_string(),
            gpu_name,
        }
    }

    /// Update and return current GPU metrics
    ///
    /// For now, returns basic provider info. Future enhancement:
    /// - NVIDIA: Use nvidia-ml-sys for utilization/memory/temp
    /// - DirectML: Use Windows APIs for memory
    /// - CoreML: Use Metal APIs for memory
    pub fn update(&mut self) -> GpuMetrics {
        // CPU provider has no GPU metrics
        if self.provider == "cpu" {
            return GpuMetrics {
                gpu_name: self.gpu_name.clone(),
                provider: self.provider.clone(),
                utilization_percent: None,
                memory_used_mb: None,
                memory_total_mb: None,
                temperature_c: None,
            };
        }

        // For CUDA/DirectML/CoreML, return basic info
        // Real metrics require platform-specific APIs (nvidia-ml-sys, Windows, Metal)
        GpuMetrics {
            gpu_name: self.gpu_name.clone(),
            provider: self.provider.clone(),
            utilization_percent: None, // Would need NVML/platform APIs
            memory_used_mb: None,      // Would need NVML/platform APIs
            memory_total_mb: None,     // Would need NVML/platform APIs
            temperature_c: None,       // Would need NVML
        }
    }

    /// Get GPU provider name
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Get GPU device name
    pub fn device_name(&self) -> &str {
        &self.gpu_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_monitor_cpu() {
        let mut monitor = GpuMonitor::new("cpu");
        assert_eq!(monitor.provider(), "cpu");

        let metrics = monitor.update();
        assert_eq!(metrics.provider, "cpu");
        assert!(metrics.utilization_percent.is_none());
    }

    #[test]
    fn test_gpu_monitor_cuda() {
        let mut monitor = GpuMonitor::new("cuda");
        assert_eq!(monitor.provider(), "cuda");

        let metrics = monitor.update();
        assert_eq!(metrics.provider, "cuda");
        assert!(metrics.gpu_name.contains("NVIDIA"));
    }
}
