use crate::process::{Process, ProcessWaiter};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::time::Duration;

use crate::generators::Input;

// Handles basic proc spawning and running under dino
// only works on 32 bit for now
pub fn get_inst_count(path: &str, inp: &Input, vars: &HashMap<String, String>) -> i64 {
    let dynpath = vars.get("dynpath").unwrap();
    let drrun = format!("{}/bin64/drrun", dynpath);
    let libinscount = format!("{}/api/bin/libinscount.so", dynpath);
    let mut proccess = Process::new(&drrun);
    proccess.arg("-c");
    proccess.arg(libinscount);
    proccess.arg("--");
    proccess.arg(path);
    for arg in inp.argv.iter() {
        proccess.arg(OsStr::from_bytes(arg));
    }

    // Start Process run it to completion with all arguements
    proccess.start().unwrap();
    proccess.write_stdin(&inp.stdin).unwrap();
    proccess.close_stdin().unwrap();
    panic!("Fix this");
    //proccess.finish(Duration::new(1, 0)).unwrap();

    let mut buf: Vec<u8> = Vec::new();
    proccess.read_stdout(&mut buf).unwrap();

    let stdout = String::from_utf8_lossy(buf.as_slice());

    let re = regex::Regex::new("Instrumentation results: (\\d+) instructions executed").unwrap();
    let caps = re.captures(&stdout).unwrap();
    let cap = &caps[caps.len() - 1];
    let num2: i64 = cap.parse().unwrap();

    num2
}
