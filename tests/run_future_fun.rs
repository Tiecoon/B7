use b7::generators::Input;
use b7::B7Opts;
use std::path::PathBuf;
use std::time::Duration;

use ctor::ctor;

static FLAG: &str = "flag{g00d_th1ng5_f0r_w41ting}";

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

#[test]
fn run_future_fun_perf() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("bins")
        .join("future_fun");

    let res = B7Opts::new(path)
        .init_input(Input {
            stdinlen: FLAG.len() as u32,
            ..Default::default()
        })
        .drop_ptrace(true)
        .solve_stdin(true)
        .timeout(Duration::from_secs(100))
        .run()
        .unwrap();

    let stdin = String::from_utf8_lossy(res.stdin.as_slice());
    assert_eq!(&stdin, FLAG);
}
