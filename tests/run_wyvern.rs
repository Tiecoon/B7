use std::path::PathBuf;
use std::collections::HashMap;
use b7::b7tui::Env;
use b7::perf::get_inst_count;
use b7::B7Opts;

#[test]
fn run_wyv() {

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("wyvern");

    let mut term = Env::new();

    let mut opts = B7Opts::new(
        path.to_string_lossy().into_owned(),
        true,
        true,
        get_inst_count,
        &mut term,
        HashMap::new()
    );

    let res = opts.run();
    let mut stdin = res.stdin_brute.unwrap();

    // Last character is currently non-deterministic
    stdin.pop();
    assert_eq!(&stdin, "dr4g0n_or_p4tric1an_it5_LLVM");
}
