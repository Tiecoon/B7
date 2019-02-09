use b7::b7tui::Env;
use b7::dynamorio;
use b7::B7Opts;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn run_wyv() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dynpath = path.clone();

    path.push("tests");
    path.push("wyvern");

    dynpath.push("dynamorio");
    dynpath.push("build");

    let mut term = Env::new();
    let mut vars = HashMap::new();
    vars.insert(
        "dynpath".to_string(),
        dynpath.to_string_lossy().into_owned(),
    );

    let mut opts = B7Opts::new(
        path.to_string_lossy().into_owned(),
        false,
        true,
        dynamorio::get_inst_count,
        &mut term,
        vars,
    );

    let res = opts.run();
    let mut stdin = res.stdin_brute.unwrap();

    // Last character is currently non-deterministic
    stdin.pop();
    assert_eq!(&stdin, "dr4g0n_or_p4tric1an_it5_LLVM");
}
