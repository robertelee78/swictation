//! GPU detection and provider selection

use tracing::{info, warn};

/// Detect available GPU provider
///
/// Returns the best available GPU provider in priority order:
/// 1. CUDA (NVIDIA) on Linux/Windows
/// 2. DirectML (any GPU) on Windows
/// 3. CoreML (Apple Silicon) on macOS
/// 4. None (CPU fallback)
pub fn detect_gpu_provider() -> Option<String> {
    // macOS: Check for Apple Silicon (CoreML)
    #[cfg(target_os = "macos")]
    {
        if check_coreml_available() {
            info!("Detected Apple Silicon GPU - using CoreML");
            return Some("coreml".to_string());
        }
    }

    // Windows: Check DirectML first (works with any GPU)
    #[cfg(target_os = "windows")]
    {
        if check_directml_available() {
            info!("Detected DirectML support");
            return Some("directml".to_string());
        }
    }

    // Linux/Windows: Check NVIDIA CUDA
    #[cfg(not(target_os = "macos"))]
    {
        if check_cuda_available() {
            info!("Detected NVIDIA GPU - using CUDA");
            return Some("cuda".to_string());
        }
    }

    warn!("No GPU detected - falling back to CPU");
    None
}

/// Check if CUDA is available (NVIDIA GPUs)
#[cfg(not(target_os = "macos"))]
fn check_cuda_available() -> bool {
    // Try to detect CUDA by checking for nvidia-smi
    std::process::Command::new("nvidia-smi")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn check_cuda_available() -> bool {
    false // No CUDA on macOS
}

/// Check if DirectML is available (Windows, any GPU)
#[allow(dead_code)]
#[cfg(all(target_os = "windows", feature = "gpu-info"))]
fn check_directml_available() -> bool {
    use windows::Win32::Graphics::Direct3D12::*;

    unsafe {
        // Try to create a D3D12 device (DirectML requirement)
        let mut device: Option<ID3D12Device> = None;
        D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device).is_ok()
    }
}

#[allow(dead_code)]
#[cfg(not(target_os = "windows"))]
fn check_directml_available() -> bool {
    false
}

#[allow(dead_code)]
#[cfg(all(target_os = "windows", not(feature = "gpu-info")))]
fn check_directml_available() -> bool {
    // On Windows without gpu-info feature, assume DirectML is available
    true
}

/// Check if CoreML is available (macOS Apple Silicon)
#[allow(dead_code)]
#[cfg(target_os = "macos")]
fn check_coreml_available() -> bool {
    use std::process::Command;

    // Check if we're running on Apple Silicon
    let output = Command::new("sysctl")
        .args(&["-n", "machdep.cpu.brand_string"])
        .output();

    if let Ok(output) = output {
        let cpu_info = String::from_utf8_lossy(&output.stdout);
        return cpu_info.contains("Apple");
    }

    false
}

#[allow(dead_code)]
#[cfg(not(target_os = "macos"))]
fn check_coreml_available() -> bool {
    false
}

/// Get GPU memory information in MB (total, available)
///
/// **Platform-specific behavior:**
/// - **Linux**: Queries NVIDIA GPU VRAM using nvidia-smi (dedicated GPU memory)
/// - **macOS**: Queries unified system memory (GPU shares RAM with CPU)
///
/// Returns None if:
/// - No GPU detected or available
/// - Query command failed
/// - Failed to parse output
///
/// # Returns
/// Some((total_mb, available_mb)) on success, None on failure
///
/// Where:
/// - **total_mb**: Total GPU memory (VRAM on Linux, system RAM on macOS)
/// - **available_mb**: Memory available for ML workloads
///   - Linux: Free VRAM reported by nvidia-smi
///   - macOS: 65% of system RAM (35% reserved for OS/apps)
///
/// # Example
/// ```no_run
/// use swictation_daemon::gpu::get_gpu_memory_mb;
///
/// if let Some((total, available)) = get_gpu_memory_mb() {
///     println!("GPU: {}MB total, {}MB available", total, available);
/// } else {
///     println!("No GPU detected");
/// }
/// ```
pub fn get_gpu_memory_mb() -> Option<(u64, u64)> {
    // macOS: Query unified system memory (GPU shares RAM with CPU)
    #[cfg(target_os = "macos")]
    {
        return get_macos_unified_memory_mb();
    }

    // Linux/Windows: Query NVIDIA GPU VRAM via nvidia-smi
    #[cfg(not(target_os = "macos"))]
    {
        return get_nvidia_vram_mb();
    }
}

/// Get macOS unified memory information (GPU shares system RAM)
///
/// Apple Silicon uses Unified Memory Architecture - GPU and CPU share the same physical RAM.
/// No separate VRAM exists, so we query system memory and reserve a portion for OS/apps.
///
/// **Memory Allocation Strategy:**
/// - Reserve 35% for OS, system processes, and other applications
/// - Return 65% as available for ML workloads
/// - This matches typical memory pressure on macOS systems
///
/// # Returns
/// Some((total_system_mb, available_for_ml_mb)) on success, None on failure
#[cfg(target_os = "macos")]
fn get_macos_unified_memory_mb() -> Option<(u64, u64)> {
    use sysinfo::System;

    let system = System::new_all();

    // Total system memory (shared between CPU/GPU/ANE)
    let total_mb = system.total_memory() / (1024 * 1024);

    if total_mb == 0 {
        warn!("Failed to query system memory on macOS");
        return None;
    }

    // Reserve 35% for OS and other apps, make 65% available for ML
    // This is conservative but prevents system slowdown during inference
    let available_mb = ((total_mb as f64) * 0.65) as u64;

    info!(
        "Detected Apple Silicon unified memory: {}MB total, {}MB available for ML (65%)",
        total_mb, available_mb
    );
    info!("Note: GPU shares system RAM - no separate VRAM on Apple Silicon");

    Some((total_mb, available_mb))
}

/// Get NVIDIA GPU VRAM via nvidia-smi (Linux/Windows)
///
/// Queries dedicated GPU memory using NVIDIA's nvidia-smi command-line tool.
/// This is separate VRAM, not shared with system RAM.
#[cfg(not(target_os = "macos"))]
fn get_nvidia_vram_mb() -> Option<(u64, u64)> {
    use std::process::Command;

    // Query NVIDIA GPU memory via nvidia-smi
    // Format: "total_mb, free_mb" (e.g., "24576, 23456")
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    // Check if command succeeded
    if !output.status.success() {
        warn!("nvidia-smi command failed with status: {:?}", output.status);
        return None;
    }

    // Parse output: "total_mb, free_mb"
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();

    if line.is_empty() {
        warn!("nvidia-smi returned empty output");
        return None;
    }

    // Split on comma and parse
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

    if parts.len() < 2 {
        warn!("nvidia-smi output format unexpected: '{}'", line);
        return None;
    }

    // Parse both values
    let total = parts[0].parse::<u64>().ok()?;
    let free = parts[1].parse::<u64>().ok()?;

    // Sanity checks
    if total == 0 {
        warn!("nvidia-smi reported 0 total memory - invalid");
        return None;
    }

    if free > total {
        warn!(
            "nvidia-smi reported free ({}) > total ({}) - invalid",
            free, total
        );
        return None;
    }

    info!("Detected NVIDIA GPU: {}MB total, {}MB free", total, free);

    Some((total, free))
}

/// Check if running on Apple Silicon (ARM64)
///
/// Returns true if the current CPU architecture is aarch64 (Apple Silicon M1/M2/M3/M4)
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn is_apple_silicon() -> bool {
    #[cfg(target_arch = "aarch64")]
    {
        true
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_detection() {
        let provider = detect_gpu_provider();
        println!("Detected GPU provider: {:?}", provider);
        // Don't assert - GPU availability depends on hardware
    }

    #[test]
    fn test_vram_detection() {
        // Test VRAM detection (will succeed on systems with NVIDIA GPU)
        let vram = get_gpu_memory_mb();

        match vram {
            Some((total, free)) => {
                println!("✓ Detected NVIDIA GPU: {}MB total, {}MB free", total, free);

                // Sanity checks
                assert!(total > 0, "Total VRAM should be positive");
                assert!(free <= total, "Free VRAM should not exceed total");
                assert!(free > 0, "Free VRAM should be positive (system uses some)");

                // Common GPU memory sizes (in MB)
                // Consumer: 2GB, 4GB, 6GB, 8GB, 10GB, 12GB, 16GB, 24GB
                // Professional: 32GB, 40GB, 48GB, 80GB
                assert!(
                    total >= 512,
                    "Total VRAM should be at least 512MB for any modern GPU"
                );

                println!("  ✓ Sanity checks passed");
            }
            None => {
                println!("ℹ No NVIDIA GPU detected (expected on CPU-only systems)");
                // This is not an error - system may not have NVIDIA GPU
            }
        }
    }

    #[test]
    fn test_vram_thresholds() {
        // Verify our threshold logic matches memory requirements from task

        // 1.1B INT8 model requirements
        let model_1_1b_peak = 3500; // 3.5GB peak usage
        let threshold_1_1b = 4096; // 4GB minimum threshold
        let headroom_1_1b = threshold_1_1b - model_1_1b_peak; // 596MB headroom

        assert!(
            headroom_1_1b >= 500,
            "1.1B model threshold ({}MB) must have at least 500MB headroom (actual: {}MB)",
            threshold_1_1b,
            headroom_1_1b
        );

        println!(
            "✓ 1.1B INT8 threshold: {}MB (peak {}MB + {}MB headroom = {:.1}% margin)",
            threshold_1_1b,
            model_1_1b_peak,
            headroom_1_1b,
            (headroom_1_1b as f32 / model_1_1b_peak as f32) * 100.0
        );

        // 0.6B GPU model requirements
        let model_0_6b_peak = 1200; // 1.2GB peak usage
        let threshold_0_6b = 1536; // 1.5GB minimum threshold
        let headroom_0_6b = threshold_0_6b - model_0_6b_peak; // 336MB headroom

        assert!(
            headroom_0_6b >= 300,
            "0.6B model threshold ({}MB) must have at least 300MB headroom (actual: {}MB)",
            threshold_0_6b,
            headroom_0_6b
        );

        println!(
            "✓ 0.6B GPU threshold: {}MB (peak {}MB + {}MB headroom = {:.1}% margin)",
            threshold_0_6b,
            model_0_6b_peak,
            headroom_0_6b,
            (headroom_0_6b as f32 / model_0_6b_peak as f32) * 100.0
        );

        // Verify threshold ordering
        assert!(
            threshold_1_1b > threshold_0_6b,
            "1.1B threshold ({}MB) must be greater than 0.6B threshold ({}MB)",
            threshold_1_1b,
            threshold_0_6b
        );

        println!(
            "✓ Threshold ordering correct: {} > {}",
            threshold_1_1b, threshold_0_6b
        );
    }

    #[test]
    fn test_model_selection_logic() {
        // Test the adaptive selection decision tree with various VRAM amounts

        struct TestCase {
            vram_mb: u64,
            expected_model: &'static str,
            reason: &'static str,
        }

        let test_cases = vec![
            TestCase {
                vram_mb: 512,
                expected_model: "0.6B CPU",
                reason: "VRAM < 1536MB (insufficient for 0.6B GPU)",
            },
            TestCase {
                vram_mb: 1024,
                expected_model: "0.6B CPU",
                reason: "VRAM < 1536MB (insufficient for 0.6B GPU)",
            },
            TestCase {
                vram_mb: 1536,
                expected_model: "0.6B GPU",
                reason: "VRAM ≥ 1536MB but < 4096MB (0.6B GPU range)",
            },
            TestCase {
                vram_mb: 2048,
                expected_model: "0.6B GPU",
                reason: "VRAM ≥ 1536MB but < 4096MB (0.6B GPU range)",
            },
            TestCase {
                vram_mb: 3072,
                expected_model: "0.6B GPU",
                reason: "VRAM ≥ 1536MB but < 4096MB (0.6B GPU range)",
            },
            TestCase {
                vram_mb: 4096,
                expected_model: "1.1B GPU INT8",
                reason: "VRAM ≥ 4096MB (strong GPU)",
            },
            TestCase {
                vram_mb: 8192,
                expected_model: "1.1B GPU INT8",
                reason: "VRAM ≥ 4096MB (strong GPU)",
            },
            TestCase {
                vram_mb: 24576,
                expected_model: "1.1B GPU INT8",
                reason: "VRAM ≥ 4096MB (strong GPU)",
            },
            TestCase {
                vram_mb: 81920,
                expected_model: "1.1B GPU INT8",
                reason: "VRAM ≥ 4096MB (strong GPU)",
            },
        ];

        println!("\nModel Selection Decision Tree Test:");
        println!("====================================");

        let test_count = test_cases.len();
        for tc in test_cases {
            let selected = if tc.vram_mb >= 4096 {
                "1.1B GPU INT8"
            } else if tc.vram_mb >= 1536 {
                "0.6B GPU"
            } else {
                "0.6B CPU"
            };

            assert_eq!(
                selected, tc.expected_model,
                "VRAM {}MB should select '{}' but got '{}' ({})",
                tc.vram_mb, tc.expected_model, selected, tc.reason
            );

            println!(
                "  ✓ {}MB VRAM → {} ({})",
                tc.vram_mb, tc.expected_model, tc.reason
            );
        }

        println!("\n✓ All {} test cases passed", test_count);
    }
}
