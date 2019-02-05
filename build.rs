extern crate bindgen;
extern crate cc;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

fn main() {
    println!("cargo:rerun-if-changed=src/bindgen.h");
    //println!("cargo:rerun-if-changed=dynamorio/");
    /*for entry in WalkDir::new("dynamorio") {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }*/

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
