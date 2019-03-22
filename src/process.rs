use crate::binary::Binary;
use crate::errors::*;
use nix::sys::ptrace;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::Pid;
use spawn_ptrace::CommandPtraceSpawn;
use std::ffi::OsStr;
use std::io::{Error, Read, Write};
use std::process::{Child, Command, Stdio};

#[derive(Debug)]
pub struct Process {
    binary: Binary,
    cmd: Command,
    child: Option<Child>,
}

// Handle running a process
impl Process {
    pub fn new(path: &str) -> Process {
        Process {
            binary: Binary::new(path),
            cmd: Command::new(path),
            child: None,
        }
    }

    pub fn child_id(&self) -> Result<u32, SolverError> {
        match &self.child {
            Some(a) => Ok(a.id()),
            None => Err(SolverError::new(Runner::IoError, "no child id")),
        }
    }

    pub fn args<I, S>(&mut self, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.cmd.args(args);
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) {
        self.cmd.arg(arg);
    }

    // initialize process and wait it
    pub fn start(&mut self) -> Result<(), SolverError> {
        if self.child.is_some() {
            return Err(SolverError::new(Runner::Unknown, "child already running"));
        }
        self.cmd.stdin(Stdio::piped());
        self.cmd.stdout(Stdio::piped());
        self.cmd.stderr(Stdio::piped());
        // spawn process and wait after fork
        let child = self.cmd.spawn_ptrace();
        match child {
            Ok(c) => {
                self.child = Some(c);
                Ok(())
            }
            Err(x) => Err(x.into()),
        }
    }

    // write buf to process then close it
    pub fn write_stdin(&mut self, buf: &[u8]) -> Result<(), SolverError> {
        if self.child.is_none() {
            return Err(SolverError::new(
                Runner::RunnerError,
                "Process is not running",
            ));
        }
        let child = self.child.as_mut().unwrap();
        match child.stdin.as_mut() {
            Some(stdin) => stdin.write_all(buf).map_err(|e| e.into()),
            None => Err(SolverError::new(Runner::IoError, "could not open stdin")),
        }
    }

    // read buf to process then close it
    pub fn read_stdout(&mut self, buf: &mut Vec<u8>) -> Result<usize, SolverError> {
        if self.child.is_none() {
            return Err(SolverError::new(
                Runner::RunnerError,
                "child process not running",
            ));
        }
        let child = self.child.as_mut().unwrap();
        match child.stdout.as_mut() {
            Some(stdout) => stdout.read_to_end(buf).map_err(|e| e.into()),
            None => Err(Error::last_os_error().into()),
        }
    }

    // close stdin to prevent any reads hanging
    pub fn close_stdin(&mut self) -> Result<(), SolverError> {
        if self.child.is_none() {
            return Err(SolverError::new(
                Runner::RunnerError,
                "child process not running",
            ));
        }
        match self.child.as_mut().unwrap().stdin.take() {
            Some(stdin) => {
                drop(stdin);
                Ok(())
            }
            None => Err(Error::last_os_error().into()),
        }
    }

    // continue executing ptrace if it is paused
    pub fn cont(&self) -> Result<(), SolverError> {
        if self.child.is_none() {
            return Err(SolverError::new(
                Runner::RunnerError,
                "child process not running",
            ));
        }
        let child = self.child.as_ref().unwrap();
        ptrace::cont(Pid::from_raw(child.id() as i32), None).map_err(|e| e.into())
        // match res {
        //     Ok(_) => Ok(()),
        //     Err(x) => Err(SolverError::new(ErrorKind::Other, format!("{:?}", x))),
        // }
    }

    // go until next pause point
    pub fn wait(&self) -> Result<WaitStatus, SolverError> {
        if self.child.is_none() {
            SolverError::new(Runner::RunnerError, "child process not running");
        }
        let child = self.child.as_ref().unwrap();
        waitpid(Pid::from_raw(child.id() as i32), None).map_err(|e| e.into())
    }

    // attempt to run the program to completion
    pub fn finish(&self) -> Result<(), SolverError> {
        loop {
            let cret = self.cont();
            if cret.is_err() {
                return cret;
            }
            let wret = self.wait();
            match wret {
                Ok(WaitStatus::Exited(_, _)) => return Ok(()),
                Err(x) => return Err(x),
                _ => (),
            }
        }
    }
}
