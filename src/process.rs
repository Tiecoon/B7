use binary::Binary;
use std::process::Command;

#[derive(Debug)]
pub struct Process {
    binary: Binary,
    program: Command
}

impl Process {
    pub fn new<S>(path: &str) -> Process {
        Process {
            binary: Binary::new(path),
            program: Command::new(path)
        }
    }
}
