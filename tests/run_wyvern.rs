use b7::b7tui::Env;
#[cfg(feature = "dynamorio")]
use b7::dynamorio;
use b7::generators::Input;
use b7::perf;
use b7::B7Opts;
use std::collections::HashMap;
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

#[cfg(feature = "dynamorio")]
#[test]
fn run_wyvern_dynamorio() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dynpath = path.clone();

    path.push("tests");
    path.push("wyvern");
    path.push("wyvern");

    let mut term = Env::new();
    let mut vars = HashMap::new();
    vars.insert(
        "dynpath".to_string(),
        dynpath.to_string_lossy().into_owned(),
    );

    let mut opts = B7Opts::new(
        path.to_string_lossy().into_owned(),
        Input::new(),
        false,
        true,
        Box::new(dynamorio::DynamorioSolver),
        &mut term,
        vars,
        Duration::new(5, 0),
    );

    let res = opts.run().unwrap();
    let mut stdin = res.stdin_brute;

    // Last character is currently non-deterministic
    stdin.pop();
    assert_eq!(&stdin, "dr4g0n_or_p4tric1an_it5_LLVM");
}

#[test]
fn run_wyvern_perf() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    path.push("tests");
    path.push("wyvern");
    path.push("wyvern");

    let mut term = Env::new();
    let vars = HashMap::new();

    let mut opts = B7Opts::new(
        path.to_string_lossy().into_owned(),
        Input::new(),
        false,
        false,
        true,
        Box::new(perf::PerfSolver),
        &mut term,
        vars,
        Duration::new(5, 0),
    );

    let mut res = opts.run().unwrap();

    res.stdin.pop();
    let stdin = String::from_utf8_lossy(res.stdin.as_slice());
    // Last character is currently non-deterministic
    assert_eq!(&stdin, "dr4g0n_or_p4tric1an_it5_LLVM");
}
