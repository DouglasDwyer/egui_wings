use std::env::*;
use std::fs::*;
use std::path::*;
use std::process::*;

/// Builds the WASM plugin and embeds it for consumption as a byte array.
fn main() {
    println!("cargo:rerun-if-changed=example_plugin");

    let out_dir = var("OUT_DIR").expect("Could not get output directory.");
    let result = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg("example_plugin")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--target-dir")
        .arg(&out_dir)
        .spawn()
        .expect("Failed to start WASI build shim.")
        .wait()
        .expect("Failed to build WASI shim.");

    if result.success() {
        let out_path = Path::new(&out_dir);
        let mut path_buf = PathBuf::from(out_path);

        path_buf.push("wasm32-unknown-unknown/release/example_plugin.wasm");
        assert!(path_buf.exists(), "Plugin not found at path: {path_buf:?}");

        let wasm = read(&path_buf).expect("Could not read WASM output.");

        write(
            out_path.join("example_plugin.rs"),
            format!("const EXAMPLE_PLUGIN_WASM: &[u8] = &{wasm:?};"),
        )
        .expect("Could not write WASM plugin bytes.");
    } else {
        panic!("Failed to generate WASM plugin.");
    }
}