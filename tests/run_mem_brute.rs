use b7::generators::Input;
use b7::generators::MemInput;
use b7::B7Opts;
use std::path::PathBuf;
use std::time::Duration;

use ctor::ctor;

// This hack ensures that we block SIGCHLD
// on every thread. When running tests,
// Rust spawns several test worker threads
// from the main thread. In order to
// ensure that *every* thread (including the main thread)
// has SIGCHLD blocked, we use the 'ctor' crate to run
// our code very early during process startup.
//
// This is not a normal function - main() has not
// yet been called, any the Rust stdlib may not yet
// be initialized. It should do the absolute minimum
// necessary to get B7 working in a test environment
#[ctor]
fn on_init() {
    b7::process::block_signal();
}

fn mem_brute_helper(mem_inputs: &[MemInput], filename: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("bins")
        .join("mem_brute")
        .join(filename);

    let mem = mem_inputs
        .iter()
        .map(|m| MemInput {
            size: m.size,
            addr: m.addr,
            bytes: Vec::new(),
            breakpoint: m.breakpoint,
        })
        .collect::<Vec<MemInput>>();

    let res = B7Opts::new(path)
        .init_input(Input {
            mem: Some(mem),
            ..Default::default()
        })
        .timeout(Duration::from_secs(5))
        .run()
        .unwrap();

    match res.mem {
        Some(mem) => assert_eq!(mem, mem_inputs),
        None => panic!("no results found"),
    }
}

#[test]
fn run_mem_brute_perf() {
    mem_brute_helper(
        &[MemInput {
            size: 26,
            addr: 0x404050,
            bytes: "ABCDEFGHIJKLMNOPQRSTUVWXYZ".bytes().collect(),
            breakpoint: None,
        }],
        "mem_brute",
    )
}

#[test]
fn run_mem_brute_pie_perf() {
    mem_brute_helper(
        &[MemInput {
            size: 26,
            addr: 0x4050,
            bytes: "ABCDEFGHIJKLMNOPQRSTUVWXYZ".bytes().collect(),
            breakpoint: None,
        }],
        "mem_brute_pie",
    )
}

#[test]
fn run_mem_brute_breakpoint_perf() {
    mem_brute_helper(
        &[MemInput {
            size: 26,
            addr: 0x404050,
            bytes: "ABCDEFGHIJKLMNOPQRSTUVWXYZ".bytes().collect(),
            breakpoint: Some(0x4011f7),
        }],
        "mem_brute_breakpoint",
    )
}

#[test]
fn run_mem_brute_breakpoint_pie_perf() {
    mem_brute_helper(
        &[MemInput {
            size: 26,
            addr: 0x4050,
            bytes: "ABCDEFGHIJKLMNOPQRSTUVWXYZ".bytes().collect(),
            breakpoint: Some(0x120a),
        }],
        "mem_brute_breakpoint_pie",
    )
}
