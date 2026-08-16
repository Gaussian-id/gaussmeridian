fn main() {
    // Configure macOS build settings
    if cfg!(target_os = "macos") {
        // Set minimum macOS version to match the system
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.12");
        
        // Configure linker settings
        println!("cargo:rustc-link-arg=-Wl,-dead_strip");
        println!("cargo:rustc-link-arg=-Wl,-platform_version");
        println!("cargo:rustc-link-arg=-Wl,macos");
        println!("cargo:rustc-link-arg=-Wl,10.12.0");
        println!("cargo:rustc-link-arg=-Wl,10.12.0");
    }

    // Configure build settings
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");

    // Configure optimization level for dependencies
    println!("cargo:rustc-env=CARGO_PROFILE_DEV_OPT_LEVEL=2");
    println!("cargo:rustc-env=CARGO_PROFILE_DEV_PACKAGE_SYN_OPT_LEVEL=2");
} 