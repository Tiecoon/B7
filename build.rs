extern crate bindgen;
extern crate cc;
extern crate num_cpus;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/bindgen.h");
    //println!("cargo:rerun-if-changed=dynamorio/");
    let mut out_dir_64 = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    out_dir_64.push("dynamorio");
    let mut out_dir_32 = out_dir_64.clone();

    out_dir_64.push("build_64");
    out_dir_32.push("build_32");

    fs::create_dir_all(&out_dir_64).expect("Failed to make build_64 dir");
    fs::create_dir_all(&out_dir_32).expect("Failed to make build_32 dir");

    let cpus = num_cpus::get();

    let run_cmake = |cmd: &mut Command| {
        if !cmd.spawn()
        .expect("Failed to spawn cmake")
        .wait()
        .expect("Failed to run cmake")
        .success() {
            panic!("cmake failed!");
        }
    };

    let run_make = |dir| {
        if !Command::new("make")
            .args(&["-j", &format!("{}", cpus)])
            .current_dir(dir)
            .spawn()
            .expect("Failed to spawn make")
            .wait()
            .expect("Failed to run make")
            .success() {
                panic!("make failed!");
        }
    };

    run_cmake(Command::new("cmake")
        .args(&["..", "-DDISABLE_WARNINGS=yes"])
        .current_dir(&out_dir_64));

    run_make(&out_dir_64);

    run_cmake(Command::new("cmake")
        .args(&["..", "-DDISABLE_WARNINGS=yes", "-DCMAKE_DISABLE_FIND_PACKAGE_Qt5Widgets=TRUE"])
        .env("CXXFLAGS", "-m32")
        .env("CFLAGS", "-m32")
        .current_dir(&out_dir_32));

    run_make(&out_dir_32);



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
