use crate::process::Process;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

use crate::generators::Input;

// Handles basic proc spawning and running under dino
// only works on 32 bit for now
pub fn get_inst_count(path: &str, inp: &Input, vars: &HashMap<String, String>) -> i64 {
    let dynpath = vars.get("dynpath").unwrap();
    let mut proc = Process::new(dynpath);
    proc.arg("-c");
    proc.arg(dynpath);
    proc.arg("--");
    proc.arg(path);
    for arg in inp.argv.iter() {
        proc.arg(OsStr::from_bytes(arg));
    }

    // Start Process run it to completion with all arguements
    proc.start().unwrap();
    proc.write_stdin(&inp.stdin).unwrap();
    proc.close_stdin().unwrap();
    proc.finish().unwrap();

    let mut buf: Vec<u8> = Vec::new();
    proc.read_stdout(&mut buf).unwrap();

    let stdout = String::from_utf8_lossy(buf.as_slice());

    let re = regex::Regex::new("(\\d+)").unwrap();
    let m = re.find(&stdout).unwrap().as_str();
    let num2: i64 = m.parse().unwrap();

    num2
}
