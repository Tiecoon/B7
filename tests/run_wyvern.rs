#[cfg(feature = "dynamorio")]
use b7::dynamorio;
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

#[cfg(feature = "dynamorio")]
#[test]
fn run_wyvern_dynamorio() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dynpath = path.clone();

    path.push("tests");
    path.push("bins");
    path.push("wyvern");

    let mut vars = HashMap::new();
    vars.insert(
        "dynpath".to_string(),
        dynpath.to_string_lossy().into_owned(),
    );

    let res = B7Opts::new(path)
        .solve_stdin(true)
        .solver(Box::new(dynamorio::DynamorioSolver))
        .vars(vars)
        .timeout(Duration::from_secs(5))
        .run()
        .unwrap();

    let mut stdin = res.stdin_brute;

    // Last character is currently non-deterministic
    stdin.pop();
    assert_eq!(&stdin, "dr4g0n_or_p4tric1an_it5_LLVM");
}

#[test]
fn run_wyvern_perf() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    path.push("tests");
    path.push("bins");
    path.push("wyvern");

    let mut res = B7Opts::new(path)
        .solve_stdin(true)
        .timeout(Duration::from_secs(5))
        .run()
        .unwrap();

    res.stdin.pop();
    let stdin = String::from_utf8_lossy(res.stdin.as_slice());
    // Last character is currently non-deterministic
    assert_eq!(&stdin, "dr4g0n_or_p4tric1an_it5_LLVM");
}
