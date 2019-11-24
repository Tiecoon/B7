#![cfg(feature = "dynamorio")]
use crate::brute::*;
use crate::errors::*;
use crate::process::Process;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use goblin::elf::header::{header32, EI_CLASS};
use goblin::elf::Elf;

#[derive(Copy, Clone)]
pub struct DynamorioSolver;

enum Arch {
    ThirtyTwo,
    SixtyFour,
}

impl DynamorioSolver {
    /// Reads the ELFCLASS of the ELF binary at the specified
    /// TODO: Support other binary formats (PE, Macho-O, etc)
    fn get_arch(&self, path: &Path) -> Result<Arch, SolverError> {
        debug!("Executing get_arch:");
        let mut f = File::open(path.canonicalize()?)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;

        let bin = Elf::parse(&buf)?;
        if bin.header.e_ident[EI_CLASS] == header32::ELFCLASS {
            Ok(Arch::ThirtyTwo)
        } else {
            Ok(Arch::SixtyFour)
        }
    }
}

impl InstCounter for DynamorioSolver {
    /// Handles basic proc spawning and running under dynamorio
    /// only works on 64 bit for now
    fn get_inst_count(&self, data: &InstCountData) -> Result<i64, SolverError> {
        debug!("Executing get_inst_count:");
        let dynpath = PathBuf::from(data.vars.get("dynpath").unwrap());

        let (build_dir, bin_dir) = match self.get_arch(&PathBuf::from(&data.path))? {
            Arch::ThirtyTwo => ("build_32", "dynamorio/bin32"),
            Arch::SixtyFour => ("build_64", "dynamorio/bin64"),
        };

        let mut base_path = dynpath.clone();
        base_path.push(build_dir);

        let mut drrun = base_path.clone();
        drrun.push(bin_dir);
        drrun.push("drrun");

        let mut libinscount = base_path.clone();
        libinscount.push("dynamorio");
        libinscount.push("api");
        libinscount.push("bin");
        libinscount.push("libinscount.so");

        let mut proccess = Process::new(&drrun)?;
        proccess.arg("-c");
        proccess.arg(libinscount);
        proccess.arg("--");
        proccess.arg(&data.path);
        if let Some(argv) = &data.inp.argv {
            for arg in argv {
                proccess.arg(OsStr::from_bytes(arg));
            }
        }
        if let Some(stdin) = &data.inp.stdin {
            proccess.stdin_input(stdin.clone());
        }

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
