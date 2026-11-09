fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=native/edge-overlay.swift");
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("edge-overlay");
        let status = std::process::Command::new("swiftc")
            .args(["native/edge-overlay.swift", "-o"])
            .arg(&output)
            .status()
            .expect("Swift compiler is required for the native macOS edge overlay");
        assert!(status.success(), "native edge overlay build failed");
        println!(
            "cargo:rustc-env=LAN_MOUSE_EDGE_OVERLAY={}",
            output.display()
        );
    }

    // composite_templates
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "lan-mouse.gresource",
    );
}
