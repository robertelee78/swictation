fn main() {
    // macOS: compile the Objective-C CoreML bridge when the coreml-native feature is enabled.
    // The `cc` crate is only available when this feature is active (it's an optional dep).
    if cfg!(target_os = "macos") && std::env::var("CARGO_FEATURE_COREML_NATIVE").is_ok() {
        #[cfg(feature = "coreml-native")]
        {
            cc::Build::new()
                .file("src/coreml_bridge.m")
                .flag("-fobjc-arc")
                .flag("-fmodules")
                .compile("coreml_bridge");

            println!("cargo:rustc-link-lib=framework=CoreML");
            println!("cargo:rustc-link-lib=framework=Foundation");
        }
    }
}
