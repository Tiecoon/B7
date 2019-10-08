use std::error;
use std::fmt;
use std::io;

pub type SolverResult<T> = Result<T, SolverError>;

#[derive(Debug)]
/// b7 errors type
pub struct SolverError {
    runner: Runner,
    message: String,
}

impl SolverError {
    pub fn new(runner: Runner, message: &str) -> SolverError {
        let message2 = message.to_string();
        SolverError {
            runner,
            message: message2,
        }
    }
}

#[derive(Debug)]
/// enum for types of errors
pub enum Runner {
    RunnerError,
    ArgError,
    IoError,
    NixError,
    Timeout,
    Unknown,
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "filler display TODO")
    }
}

impl error::Error for SolverError {
    fn description(&self) -> &str {
        &self.message
    }
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

// TODO: Make this better
impl From<goblin::error::Error> for SolverError {
    fn from(error: goblin::error::Error) -> Self {
        SolverError::new(Runner::IoError, error::Error::description(&error))
    }
}

impl From<io::Error> for SolverError {
    fn from(error: io::Error) -> Self {
        SolverError::new(Runner::IoError, error::Error::description(&error))
    }
}

impl From<nix::Error> for SolverError {
    fn from(error: nix::Error) -> Self {
        SolverError::new(Runner::NixError, error::Error::description(&error))
    }
}
