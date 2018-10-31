use binary::Binary;
use std::process::Command;
use std::ffi::OsStr;

#[derive(Debug)]
pub struct Process {
    binary: Binary,
    program: Command
}

impl Process {
    pub fn new(path: &str) -> Process {
        Process {
            binary: Binary::new(path),
            program: Command::new(path)
        }
    }
    pub fn args<I, S>(&mut self, args: I)
        where I: IntoIterator<Item = S>, S: AsRef<OsStr>
    {
        self.program.args(args);
    }
}
