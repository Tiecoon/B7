use crate::errors::*;
use crate::process::Process;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

use crate::generators::Input;

// Handles basic proc spawning and running under dino
// only works on 32 bit for now
pub fn get_inst_count(
    path: &str,
    inp: &Input,
    vars: &HashMap<String, String>,
) -> Result<i64, SolverError> {
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
    proccess.start()?;
    proccess.write_stdin(&inp.stdin)?;
    proccess.close_stdin()?;
    proccess.finish()?;

    let mut buf: Vec<u8> = Vec::new();
    proccess.read_stdout(&mut buf)?;

    let stdout = String::from_utf8_lossy(buf.as_slice());

    let re = regex::Regex::new("Instrumentation results: (\\d+) instructions executed").unwrap();
    let caps = match re.captures(&stdout) {
        Some(x) => x,
        None => {
            return Err(SolverError::new(
                Runner::IoError,
                "Could not parse dynamorio Instruction count",
            ));
        }
    };
    let cap = &caps[caps.len() - 1];
    let num2: i64 = cap.parse().unwrap();

    Ok(num2)
}
