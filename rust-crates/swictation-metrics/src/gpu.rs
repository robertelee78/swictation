//! GPU monitoring module
//!
//! Provides GPU metrics with platform-specific detection

use serde::{Deserialize, Serialize};

#[cfg(all(target_os = "macos", feature = "gpu-monitoring"))]
use metal::{Device, MTLResourceOptions};

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
    /// Platform-specific implementations:
    /// - macOS: Uses Metal framework to query unified memory
    /// - NVIDIA: Future enhancement with nvidia-ml-sys
    /// - DirectML: Future enhancement with Windows APIs
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

        // macOS: Use Metal APIs for unified memory metrics
        #[cfg(all(target_os = "macos", feature = "gpu-monitoring"))]
        if self.provider == "coreml" {
            return self.get_macos_gpu_metrics();
        }

        // For CUDA/DirectML, return basic info for now
        // Real metrics require platform-specific APIs (nvidia-ml-sys, Windows APIs)
        GpuMetrics {
            gpu_name: self.gpu_name.clone(),
            provider: self.provider.clone(),
            utilization_percent: None, // Would need NVML/platform APIs
            memory_used_mb: None,      // Would need NVML/platform APIs
            memory_total_mb: None,     // Would need NVML/platform APIs
            temperature_c: None,       // Would need NVML
        }
    }

    /// Get GPU metrics on macOS using Metal framework
    ///
    /// Queries unified memory architecture:
    /// - recommendedMaxWorkingSetSize: Recommended memory limit for GPU
    /// - currentAllocatedSize: Currently allocated GPU memory
    ///
    /// macOS unified memory: CPU and GPU share system RAM
    /// GPU gets ~35% of total RAM (65/35 split from system memory)
    #[cfg(all(target_os = "macos", feature = "gpu-monitoring"))]
    fn get_macos_gpu_metrics(&self) -> GpuMetrics {
        let device = Device::system_default();

        match device {
            Some(device) => {
                // Get recommended working set size (effective GPU memory available)
                let recommended_mb = device.recommended_max_working_set_size() / (1024 * 1024);

                // Get current allocated size
                let allocated_mb = device.current_allocated_size() / (1024 * 1024);

                // Get device name from Metal
                let device_name = device.name();

                GpuMetrics {
                    gpu_name: device_name.to_string(),
                    provider: self.provider.clone(),
                    utilization_percent: None, // Metal doesn't expose real-time utilization
                    memory_used_mb: Some(allocated_mb),
                    memory_total_mb: Some(recommended_mb),
                    temperature_c: None, // Not exposed by Metal framework
                }
            }
            None => {
                // Fallback if Metal device not available
                GpuMetrics {
                    gpu_name: self.gpu_name.clone(),
                    provider: self.provider.clone(),
                    utilization_percent: None,
                    memory_used_mb: None,
                    memory_total_mb: None,
                    temperature_c: None,
                }
            }
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
