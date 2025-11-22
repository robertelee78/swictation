//! Version information and build metadata display

use std::fmt;

/// Comprehensive version information for swictation
pub struct VersionInfo {
    /// Package version from Cargo.toml
    pub daemon_version: &'static str,
    /// ONNX Runtime version
    pub ort_version: &'static str,
    /// Target triple (e.g., x86_64-unknown-linux-gnu)
    pub target: &'static str,
    /// Build profile (debug or release)
    pub profile: &'static str,
    /// Enabled features
    pub features: Vec<&'static str>,
    /// Git commit hash (if available)
    pub git_commit: Option<&'static str>,
    /// Build timestamp
    pub build_timestamp: &'static str,
}

impl VersionInfo {
    /// Get current version information
    pub fn current() -> Self {
        let mut features = Vec::new();

        #[cfg(feature = "sway-integration")]
        features.push("sway-integration");

        #[cfg(feature = "gpu-info")]
        features.push("gpu-info");

        #[cfg(feature = "minimal")]
        features.push("minimal");

        // Get ORT version if available
        let ort_version = Self::get_ort_version();

        Self {
            daemon_version: env!("CARGO_PKG_VERSION"),
            ort_version,
            target: env!("TARGET"),
            profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            },
            features,
            git_commit: option_env!("GIT_COMMIT_HASH"),
            build_timestamp: env!("BUILD_TIMESTAMP"),
        }
    }

    /// Attempt to get ONNX Runtime version
    fn get_ort_version() -> &'static str {
        // The ort crate doesn't expose version at runtime easily,
        // so we use the compile-time version
        env!("ORT_VERSION")
    }

    /// Get model compatibility information
    pub fn model_compatibility(&self) -> Vec<&'static str> {
        vec!["Parakeet-TDT-0.6B-V3 (ONNX)", "Parakeet-TDT-1.1B-V3 (ONNX)"]
    }

    /// Get GPU library compatibility
    pub fn gpu_libraries(&self) -> Vec<&'static str> {
        vec![
            "CUDA 11.8 + cuDNN 8.9.7 (Maxwell/Pascal/Volta)",
            "CUDA 12.9 + cuDNN 9.15.1 (Turing/Ampere/Ada/Hopper/Blackwell)",
        ]
    }
}

impl fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "swictation-daemon {}", self.daemon_version)?;
        writeln!(f)?;

        writeln!(f, "Build Information:")?;
        writeln!(f, "  ONNX Runtime: {}", self.ort_version)?;
        writeln!(f, "  Target:       {}", self.target)?;
        writeln!(f, "  Profile:      {}", self.profile)?;
        writeln!(f, "  Build Date:   {}", self.build_timestamp)?;

        if let Some(commit) = self.git_commit {
            writeln!(f, "  Git Commit:   {}", commit)?;
        }

        if !self.features.is_empty() {
            writeln!(f, "  Features:     {}", self.features.join(", "))?;
        }

        writeln!(f)?;
        writeln!(f, "Model Compatibility:")?;
        for model in self.model_compatibility() {
            writeln!(f, "  • {}", model)?;
        }

        writeln!(f)?;
        writeln!(f, "GPU Libraries:")?;
        for lib in self.gpu_libraries() {
            writeln!(f, "  • {}", lib)?;
        }

        Ok(())
    }
}

/// Short version string (for --version)
pub fn version_short() -> String {
    format!("swictation-daemon {}", env!("CARGO_PKG_VERSION"))
}

/// Long version string (for --version --verbose or similar)
pub fn version_long() -> String {
    VersionInfo::current().to_string()
}
