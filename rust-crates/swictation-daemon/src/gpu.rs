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
#[cfg(all(target_os = "windows", feature = "gpu-info"))]
fn check_directml_available() -> bool {
    use windows::Win32::Graphics::Direct3D12::*;

    unsafe {
        // Try to create a D3D12 device (DirectML requirement)
        let mut device: Option<ID3D12Device> = None;
        D3D12CreateDevice(
            None,
            D3D_FEATURE_LEVEL_11_0,
            &mut device,
        ).is_ok()
    }
}

#[cfg(not(target_os = "windows"))]
fn check_directml_available() -> bool {
    false
}

#[cfg(all(target_os = "windows", not(feature = "gpu-info")))]
fn check_directml_available() -> bool {
    // On Windows without gpu-info feature, assume DirectML is available
    true
}

/// Check if CoreML is available (macOS Apple Silicon)
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

#[cfg(not(target_os = "macos"))]
fn check_coreml_available() -> bool {
    false
}

/// Get GPU memory information (if available)
pub fn get_gpu_memory_mb() -> Option<(u64, u64)> {
    // GPU memory info requires platform-specific APIs
    // For now, return None - this can be implemented later with proper GPU info features
    None
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
}
