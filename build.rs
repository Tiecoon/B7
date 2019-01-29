extern crate bindgen;
extern crate cc;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mut out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    out_dir.push("dynamorio");
    out_dir.push("build");

    fs::create_dir_all(&out_dir).expect("Failed to make output dir");

    Command::new("cmake")
        .args(&[".."])
        .current_dir(&out_dir)
        .spawn()
        .expect("Failed to spawn cmake")
        .wait()
        .expect("Failed to run cmake");

    Command::new("make")
        .current_dir(out_dir)
        .spawn()
        .expect("Failed to spawn make")
        .wait()
        .expect("Failed to run make");

    // Generate Rust bindings
    let bindings = bindgen::Builder::default()
        .header("src/bindgen.h")
        .whitelist_type("perf_event_attr")
        .whitelist_type("perf_type_id")
        .whitelist_type("perf_hw_id")
        .generate()
        .expect("Unable to generate bindings");

    // Output rust bindings to a file
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
