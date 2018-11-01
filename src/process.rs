use binary::Binary;
use libc;
use libc::{c_int, c_void, pid_t};
use nix;
use nix::sys::ptrace;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::Pid;
use spawn_ptrace::CommandPtraceSpawn;
use std::ffi::OsStr;
use std::io::{Error, ErrorKind, Result, Write};
use std::process::{Child, Command, Stdio};

#[derive(Debug)]
pub struct Process {
    binary: Binary,
    cmd: Command,
    child: Option<Child>,
    perf_fd: c_int,
}

impl Process {
    pub fn new(path: &str) -> Process {
        Process {
            binary: Binary::new(path),
            cmd: Command::new(path),
            child: None,
            perf_fd: -1,
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

    pub fn start(&mut self) -> Result<()> {
        if let Some(_) = self.child {
            return Err(Error::new(
                ErrorKind::Other,
                "child process already running",
            ));
        }
        self.cmd.stdin(Stdio::piped());
        self.cmd.stdout(Stdio::piped());
        self.cmd.stderr(Stdio::piped());
        let child = self.cmd.spawn_ptrace();
        match child {
            Ok(c) => {
                self.child = Some(c);
                Ok(())
            }
            Err(c) => Err(c),
        }
    }

    pub fn write_stdin(&mut self, buf: &[u8]) -> Result<()> {
        if let None = self.child {
            return Err(Error::new(ErrorKind::Other, "child process not running"));
        }
        let child = self.child.as_mut().unwrap();
        match child.stdin.as_mut() {
            Some(stdin) => stdin.write_all(buf),
            None => Err(Error::last_os_error()),
        }
    }

    pub fn close_stdin(&mut self) -> Result<()> {
        if let None = self.child {
            return Err(Error::new(ErrorKind::Other, "child process not running"));
        }
        match self.child.as_mut().unwrap().stdin.take() {
            Some(stdin) => {
                drop(stdin);
                Ok(())
            }
            None => Err(Error::last_os_error()),
        }
    }

    pub fn init_perf(&mut self) -> Result<()> {
        extern "C" {
            fn get_perf_fd(input: pid_t) -> c_int;
        }
        match self.child {
            None => Err(Error::new(ErrorKind::Other, "child process not running")),
            Some(ref child) => unsafe {
                self.perf_fd = get_perf_fd(pid_t::from(child.id() as i32));
                Ok(())
            },
        }
    }

    pub fn cont(&self) -> Result<()> {
        if let None = self.child {
            return Err(Error::new(ErrorKind::Other, "child process not running"));
        }
        let child = self.child.as_ref().unwrap();
        let res = ptrace::cont(Pid::from_raw(pid_t::from(child.id() as i32)), None);
        match res {
            Ok(x) => Ok(x),
            Err(x) => Err(Error::new(ErrorKind::Other, format!("{:?}", x))),
        }
    }

    pub fn wait(&self) -> Result<WaitStatus> {
        if let None = self.child {
            return Err(Error::new(ErrorKind::Other, "child process not running"));
        }
        let child = self.child.as_ref().unwrap();
        match waitpid(Pid::from_raw(pid_t::from(child.id() as i32)), None) {
            Err(x) => Err(Error::new(ErrorKind::Other, format!("{:?}", x))),
            Ok(x) => Ok(x),
        }
    }

    pub fn finish(&self) -> Result<()> {
        loop {
            let cret = self.cont();
            if let Err(_) = cret {
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

    pub fn get_inst_count(&self) -> Result<i64> {
        let mut count: i64 = 0;
        let count_p = &mut count as *mut i64;
        let nread: i64;
        unsafe {
            nread = libc::read(self.perf_fd, count_p as *mut c_void, 8) as i64;
        };
        match nread {
            8 => Ok(count),
            x if x >= 0 => Err(Error::new(ErrorKind::Other, nread.to_string())),
            _ => Err(Error::new(ErrorKind::Other, nix::Error::last())),
        }
    }
}
