extern crate bindgen;

#[cfg(linux)]
use std::env;
#[cfg(linux)]
use std::path::PathBuf;

#[cfg(linux)]
fn main() {
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

#[cfg(not(linux))]
fn main() {}
