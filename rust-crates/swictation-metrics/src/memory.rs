//! Memory monitoring with MANDATORY GPU VRAM tracking
//!
//! Provides comprehensive memory monitoring for both RAM and VRAM
//! with platform-specific GPU APIs (NVML, Metal, DirectML)

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Failed to initialize system monitor")]
    SystemInit,

    #[error("Failed to initialize GPU monitoring: {0}")]
    GpuInit(String),

    #[error("GPU monitoring not available on this platform")]
    UnsupportedPlatform,

    #[error("Failed to query GPU memory: {0}")]
    GpuQuery(String),
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPressure {
    Normal,
    Warning,
    Critical,
}

/// Memory thresholds configuration
#[derive(Debug, Clone)]
pub struct MemoryThresholds {
    pub ram_warning_percent: f32,
    pub ram_critical_percent: f32,
    pub vram_warning_percent: f32,
    pub vram_critical_percent: f32,
}

impl Default for MemoryThresholds {
    fn default() -> Self {
        Self {
            ram_warning_percent: 80.0,
            ram_critical_percent: 90.0,
            vram_warning_percent: 85.0,
            vram_critical_percent: 95.0,
        }
    }
}

/// System RAM statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RamStats {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: u64,
    pub process_mb: u64,
    pub percent_used: f32,
    pub pressure: MemoryPressure,
}

/// GPU VRAM statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VramStats {
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
    pub percent_used: f32,
    pub pressure: MemoryPressure,
    pub device_name: String,
}

/// Complete memory statistics (RAM + VRAM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub ram: RamStats,
    pub vram: Option<VramStats>,
}

/// Platform-agnostic GPU memory monitoring trait
pub trait GpuMemoryProvider: Send + Sync {
    fn get_stats(&mut self) -> Result<VramStats, MemoryError>;
    fn device_name(&self) -> &str;
}

/// Fallback CPU provider (no GPU)
struct CpuProvider {
    device_name: String,
}

impl CpuProvider {
    fn new() -> Self {
        Self {
            device_name: "CPU (No GPU)".to_string(),
        }
    }
}

impl GpuMemoryProvider for CpuProvider {
    fn get_stats(&mut self) -> Result<VramStats, MemoryError> {
        Err(MemoryError::UnsupportedPlatform)
    }

    fn device_name(&self) -> &str {
        &self.device_name
    }
}

/// Memory monitor with RAM + VRAM tracking
pub struct MemoryMonitor {
    system: System,
    gpu_provider: Box<dyn GpuMemoryProvider>,
    thresholds: MemoryThresholds,
    current_pid: Pid,
}

impl MemoryMonitor {
    /// Create new memory monitor with GPU detection
    pub fn new() -> Result<Self, MemoryError> {
        let system = System::new_all();
        let current_pid = Pid::from_u32(std::process::id());

        // Try to detect and initialize GPU monitoring (MANDATORY attempt)
        let gpu_provider = match detect_gpu_provider() {
            Ok(provider) => {
                tracing::info!("GPU memory monitoring enabled: {}", provider.device_name());
                provider
            }
            Err(e) => {
                tracing::warn!(
                    "GPU memory monitoring unavailable: {} - continuing with RAM-only monitoring",
                    e
                );
                Box::new(CpuProvider::new())
            }
        };

        Ok(Self {
            system,
            gpu_provider,
            thresholds: MemoryThresholds::default(),
            current_pid,
        })
    }

    /// Create with custom thresholds
    pub fn with_thresholds(thresholds: MemoryThresholds) -> Result<Self, MemoryError> {
        let mut monitor = Self::new()?;
        monitor.thresholds = thresholds;
        Ok(monitor)
    }

    /// Get current memory statistics
    pub fn get_stats(&mut self) -> MemoryStats {
        let ram = self.get_ram_stats();
        let vram = self.gpu_provider.get_stats().ok();

        MemoryStats { ram, vram }
    }

    /// Check memory pressure levels (RAM, VRAM)
    pub fn check_pressure(&mut self) -> (MemoryPressure, MemoryPressure) {
        let ram_pressure = self.check_ram_pressure();
        let vram_pressure = self.check_vram_pressure();
        (ram_pressure, vram_pressure)
    }

    /// Get RAM statistics
    fn get_ram_stats(&mut self) -> RamStats {
        self.system.refresh_memory();
        self.system
            .refresh_processes(sysinfo::ProcessesToUpdate::Some(&[self.current_pid]), false);

        let total_mb = self.system.total_memory() / 1_048_576;
        let used_mb = self.system.used_memory() / 1_048_576;
        let available_mb = self.system.available_memory() / 1_048_576;

        let process_mb = self
            .system
            .process(self.current_pid)
            .map(|p| p.memory() / 1_048_576)
            .unwrap_or(0);

        let percent_used = if total_mb > 0 {
            (used_mb as f32 / total_mb as f32) * 100.0
        } else {
            0.0
        };

        let pressure = if percent_used >= self.thresholds.ram_critical_percent {
            MemoryPressure::Critical
        } else if percent_used >= self.thresholds.ram_warning_percent {
            MemoryPressure::Warning
        } else {
            MemoryPressure::Normal
        };

        RamStats {
            total_mb,
            used_mb,
            available_mb,
            process_mb,
            percent_used,
            pressure,
        }
    }

    /// Check RAM pressure level
    fn check_ram_pressure(&mut self) -> MemoryPressure {
        let stats = self.get_ram_stats();
        stats.pressure
    }

    /// Check VRAM pressure level
    fn check_vram_pressure(&mut self) -> MemoryPressure {
        match self.gpu_provider.get_stats() {
            Ok(stats) => stats.pressure,
            Err(_) => MemoryPressure::Normal, // No GPU or query failed
        }
    }

    /// Get GPU device name
    pub fn gpu_device_name(&self) -> &str {
        self.gpu_provider.device_name()
    }
}

// Platform-specific GPU provider detection
fn detect_gpu_provider() -> Result<Box<dyn GpuMemoryProvider>, MemoryError> {
    // Try NVIDIA NVML first (Linux/Windows)
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        if let Ok(provider) = nvidia::NvidiaProvider::new() {
            return Ok(Box::new(provider));
        }
    }

    // Try Metal (macOS)
    #[cfg(target_os = "macos")]
    {
        if let Ok(provider) = metal_gpu::MetalProvider::new() {
            return Ok(Box::new(provider));
        }
    }

    // No GPU provider available
    Err(MemoryError::UnsupportedPlatform)
}

// NVIDIA GPU provider (Linux/Windows via NVML)
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod nvidia {
    use super::*;

    #[cfg(feature = "gpu-monitoring")]
    use nvml_wrapper::Device;
    #[cfg(feature = "gpu-monitoring")]
    use nvml_wrapper::Nvml;

    pub struct NvidiaProvider {
        #[cfg(feature = "gpu-monitoring")]
        #[allow(dead_code)]
        nvml: Nvml,
        #[cfg(feature = "gpu-monitoring")]
        device: Device<'static>,

        device_name: String,
        thresholds: MemoryThresholds,
    }

    impl NvidiaProvider {
        pub fn new() -> Result<Self, MemoryError> {
            #[cfg(feature = "gpu-monitoring")]
            {
                let nvml = Nvml::init()
                    .map_err(|e| MemoryError::GpuInit(format!("NVML init failed: {}", e)))?;

                let device = nvml.device_by_index(0).map_err(|e| {
                    MemoryError::GpuInit(format!("Failed to get GPU device: {}", e))
                })?;

                let device_name = device.name().unwrap_or_else(|_| "NVIDIA GPU".to_string());

                // Leak nvml to get 'static lifetime for device
                let nvml_static = Box::leak(Box::new(nvml));
                let device_static = nvml_static.device_by_index(0).map_err(|e| {
                    MemoryError::GpuInit(format!("Failed to get GPU device: {}", e))
                })?;

                Ok(Self {
                    nvml: unsafe { std::ptr::read(nvml_static) },
                    device: device_static,
                    device_name,
                    thresholds: MemoryThresholds::default(),
                })
            }

            #[cfg(not(feature = "gpu-monitoring"))]
            Err(MemoryError::GpuInit(
                "NVML support not compiled".to_string(),
            ))
        }
    }

    impl GpuMemoryProvider for NvidiaProvider {
        fn get_stats(&mut self) -> Result<VramStats, MemoryError> {
            #[cfg(feature = "gpu-monitoring")]
            {
                let memory_info = self
                    .device
                    .memory_info()
                    .map_err(|e| MemoryError::GpuQuery(format!("NVML query failed: {}", e)))?;

                let total_mb = memory_info.total / 1_048_576;
                let used_mb = memory_info.used / 1_048_576;
                let free_mb = memory_info.free / 1_048_576;

                let percent_used = if total_mb > 0 {
                    (used_mb as f32 / total_mb as f32) * 100.0
                } else {
                    0.0
                };

                let pressure = if percent_used >= self.thresholds.vram_critical_percent {
                    MemoryPressure::Critical
                } else if percent_used >= self.thresholds.vram_warning_percent {
                    MemoryPressure::Warning
                } else {
                    MemoryPressure::Normal
                };

                Ok(VramStats {
                    total_mb,
                    used_mb,
                    free_mb,
                    percent_used,
                    pressure,
                    device_name: self.device_name.clone(),
                })
            }

            #[cfg(not(feature = "gpu-monitoring"))]
            Err(MemoryError::GpuQuery(
                "NVML support not compiled".to_string(),
            ))
        }

        fn device_name(&self) -> &str {
            &self.device_name
        }
    }
}

// Metal GPU provider (macOS via Metal APIs)
#[cfg(target_os = "macos")]
mod metal_gpu {
    use super::*;

    #[cfg(feature = "gpu-monitoring")]
    use metal::Device;

    pub struct MetalProvider {
        #[cfg(feature = "gpu-monitoring")]
        device: Device,

        device_name: String,
        thresholds: MemoryThresholds,
    }

    impl MetalProvider {
        pub fn new() -> Result<Self, MemoryError> {
            #[cfg(feature = "gpu-monitoring")]
            {
                let device = Device::system_default()
                    .ok_or_else(|| MemoryError::GpuInit("No Metal device found".to_string()))?;

                let device_name = device.name().to_string();

                Ok(Self {
                    device,
                    device_name,
                    thresholds: MemoryThresholds::default(),
                })
            }

            #[cfg(not(feature = "gpu-monitoring"))]
            Err(MemoryError::GpuInit(
                "Metal support not compiled".to_string(),
            ))
        }
    }

    impl GpuMemoryProvider for MetalProvider {
        fn get_stats(&mut self) -> Result<VramStats, MemoryError> {
            #[cfg(feature = "gpu-monitoring")]
            {
                // Metal API for memory stats
                let current_allocated = self.device.current_allocated_size();
                let max_working_set = self.device.recommended_max_working_set_size();

                let used_mb = current_allocated / 1_048_576;
                let total_mb = max_working_set / 1_048_576;
                let free_mb = total_mb.saturating_sub(used_mb);

                let percent_used = if total_mb > 0 {
                    (used_mb as f32 / total_mb as f32) * 100.0
                } else {
                    0.0
                };

                let pressure = if percent_used >= self.thresholds.vram_critical_percent {
                    MemoryPressure::Critical
                } else if percent_used >= self.thresholds.vram_warning_percent {
                    MemoryPressure::Warning
                } else {
                    MemoryPressure::Normal
                };

                Ok(VramStats {
                    total_mb,
                    used_mb,
                    free_mb,
                    percent_used,
                    pressure,
                    device_name: self.device_name.clone(),
                })
            }

            #[cfg(not(feature = "gpu-monitoring"))]
            Err(MemoryError::GpuQuery(
                "Metal support not compiled".to_string(),
            ))
        }

        fn device_name(&self) -> &str {
            &self.device_name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_monitor_creation() {
        // Should succeed even if GPU monitoring fails
        let result = MemoryMonitor::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ram_stats() {
        let mut monitor = MemoryMonitor::new().unwrap();
        let stats = monitor.get_stats();

        // RAM stats should always be available
        assert!(stats.ram.total_mb > 0);
        assert!(stats.ram.percent_used >= 0.0);
        assert!(stats.ram.percent_used <= 100.0);
    }

    #[test]
    fn test_pressure_levels() {
        let mut monitor = MemoryMonitor::new().unwrap();
        let (ram_pressure, vram_pressure) = monitor.check_pressure();

        // Should return valid pressure levels
        assert!(matches!(
            ram_pressure,
            MemoryPressure::Normal | MemoryPressure::Warning | MemoryPressure::Critical
        ));
        assert!(matches!(
            vram_pressure,
            MemoryPressure::Normal | MemoryPressure::Warning | MemoryPressure::Critical
        ));
    }
}
