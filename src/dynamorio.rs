use crate::brute::*;
use crate::errors::*;
use crate::process::Process;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

#[derive(Copy, Clone)]
pub struct DynamorioSolver;

impl InstCounter for DynamorioSolver {
    // Handles basic proc spawning and running under dino
    // only works on 64 bit for now
    fn get_inst_count(&self, data: &InstCountData) -> Result<i64, SolverError> {
        let dynpath = data.vars.get("dynpath").unwrap();
        let drrun = format!("{}/bin64/drrun", dynpath);
        let libinscount = format!("{}/api/bin/libinscount.so", dynpath);
        let mut proccess = Process::new(&drrun);
        proccess.arg("-c");
        proccess.arg(libinscount);
        proccess.arg("--");
        proccess.arg(&data.path);
        for arg in data.inp.argv.iter() {
            proccess.arg(OsStr::from_bytes(arg));
        }
        proccess.input(data.inp.stdin.clone());

        let mut handle = proccess.spawn();
        handle.finish(data.timeout)?;

        let mut buf: Vec<u8> = Vec::new();
        handle.read_stdout(&mut buf)?;

        let stdout = String::from_utf8_lossy(buf.as_slice());

        let re =
            regex::Regex::new("Instrumentation results: (\\d+) instructions executed").unwrap();
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
}
