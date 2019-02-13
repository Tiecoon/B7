use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct SolverError {
    runner: Runner,
    message: String,
}

impl SolverError {
    pub fn new(runner: Runner, message: String) -> SolverError {
        SolverError { runner, message }
    }
}

#[derive(Debug)]
pub enum Runner {
    Perf,
    Dynamorio,
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "filler display TODO")
    }
}

impl Error for SolverError {
    fn description(&self) -> &str {
        &self.message
    }
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
