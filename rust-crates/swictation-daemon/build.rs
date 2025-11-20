// Build script to capture version information and build metadata

use std::env;
use std::process::Command;

fn main() {
    // Capture target triple
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=TARGET={}", target);

    // Capture build timestamp
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);

    // Try to capture git commit hash
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=GIT_COMMIT_HASH={}", commit);
        }
    }

    // Get ONNX Runtime version from ort crate
    // Since we can't easily get this at runtime, we hardcode the version we're using
    println!("cargo:rustc-env=ORT_VERSION=1.23.2");

    // Re-run if git HEAD changes
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}
