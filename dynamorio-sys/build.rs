use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {

    println!("cargo:rerun-if-changed=src/bindings.h");
    println!("cargo:rustc-link-lib=static=dynamorio_static");
    println!("cargo:rustc-link-lib=static=drhelper");
    println!("cargo:rustc-link-lib=static=drinjectlib");
    println!("cargo:rustc-link-lib=static=drconfiglib");
    println!("cargo:rustc-link-lib=static=drfrontendlib");
    println!("cargo:rustc-link-search=dynamorio/build/lib64");
    println!("cargo:rustc-link-search=dynamorio/build/lib64/release");

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

    let bindings = bindgen::Builder::default()
        .header("src/bindings.h")
        .blacklist_item("FP_NAN")
        .blacklist_item("FP_INFINITE")
        .blacklist_item("FP_ZERO")
        .blacklist_item("FP_SUBNORMAL")
        .blacklist_item("FP_NORMAL")
        .generate()
        .expect("Unable to generate bindings");

    // Output rust bindings to a file
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");


}
