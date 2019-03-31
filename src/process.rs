use crate::binary::Binary;
use crate::errors::*;
use nix::sys::ptrace;
use nix::sys::signal::{self, SigSet, Signal, SigmaskHow};
use nix::sys::wait::{waitpid, WaitStatus, WaitPidFlag};
use nix::errno::Errno;
use nix::unistd::Pid;
use std::ffi::OsStr;
use std::io::{Error, Read, Write};
use std::process::{Child, Command, Stdio};
use std::os::unix::process::CommandExt;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::time::{Duration, Instant};
use std::convert::Into;
use std::thread::{self, ThreadId};
use lazy_static::lazy_static;

use crate::siginfo::better_siginfo_t;

#[derive(Debug)]
pub struct SignalData {
    //pub info: better_siginfo_t,
    // parsed from 'info' filed
    //pub status: SignalStatus,
    pub status: WaitStatus,
    pub pid: Pid
}


#[derive(Debug, Clone)]
pub enum SignalStatus {
    Exited(Sender<SignalData>),

    Other
}

// All of the raw pointers in this type
// are filled in with address by Linux,
// to provide information about the signal.
// We never actually try to deference them


lazy_static! {
    pub static ref WAITER: ProcessWaiter = {
        ProcessWaiter::new()
    };
}

// There is exactly one ProcessWaiter for the entire
// process.
// ProcessWaiter needs complete over signal handling
// for the process, so multiple cannot ever coexist


pub struct ProcessWaiter {
    started: bool,
    inner: Arc<Mutex<ProcessWaiterInner>>,
    initialized: Mutex<HashSet<ThreadId>>,
}

struct ChanPair {
    sender: Sender<SignalData>,
    receiver: Option<Receiver<SignalData>>
}

impl ChanPair {

    fn new() -> ChanPair {
        let (sender, receiver) = channel();
        ChanPair {
            sender,
            receiver: Some(receiver)
        }
    }

    fn take_recv(&mut self) -> Receiver<SignalData> {
        self.receiver.take().expect("Already took receiver!")
    }
}

struct ProcessWaiterInner {
    proc_chans: HashMap<Pid, ChanPair>,
}

impl ProcessWaiter {
    pub fn new() -> ProcessWaiter {
        let mut waiter = ProcessWaiter {
            inner: Arc::new(Mutex::new(ProcessWaiterInner {
                proc_chans: HashMap::new()
            })),
            started: false,
            initialized: Mutex::new(HashSet::new()),
        };
        waiter.block_signal();
        waiter.start_thread();
        waiter
    }

    pub fn start_thread(&mut self) {
        if self.started {
            panic!("Already started waiter thread!");
        }
        self.started = true;
        ProcessWaiter::spawn_waiting_thread(self.inner.clone());
    }

    pub fn block_signal(&self) {
        let mut mask = SigSet::empty();
        mask.add(Signal::SIGCHLD);

        signal::pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&mask), None).expect("Failed to block signals!");

    }

    // Block SIGCHLD for the calling thread
    // Records the initialization for the thread
    pub fn init_for_thread(&self) {
        self.block_signal();
        //println!("Init for thread!");

        self.initialized.lock().unwrap().insert(thread::current().id());
    }

    pub fn spawn_process(&self, mut process: Process) -> ProcessHandle  {
        let mut recv;
        process.start().expect("Failed to spawn process!");
        process.write_input().unwrap();
        process.close_stdin().unwrap();


        let pid = Pid::from_raw(process.child_id().unwrap() as i32);

        {
            // Critical section - create channel pair if it does
            // not exist, and take the receiver end
            let proc_chans = &mut self.inner.lock().unwrap().proc_chans;

            recv = proc_chans.entry(pid)
                .or_insert_with(ChanPair::new)
                .take_recv();
        }
        ProcessHandle { pid, recv, inner: self.inner.clone(), proc: process }
    }

    fn spawn_waiting_thread(waiter_lock: Arc<Mutex<ProcessWaiterInner>>) {
        assert_eq!(std::mem::size_of::<libc::siginfo_t>(), std::mem::size_of::<better_siginfo_t>());
        std::thread::spawn(move || {

            let mut chld_mask = SigSet::empty();
            chld_mask.add(Signal::SIGCHLD);
            signal::pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&chld_mask), None).unwrap();

            let mask = SigSet::all();
            let mut info: better_siginfo_t = unsafe { std::mem::zeroed() };

            let sigset_ptr = mask.as_ref() as *const libc::sigset_t;
            // Safe because we defined better_siginfo_t, to be compatible with libc::siginfo_t
            let info_ptr = unsafe { std::mem::transmute::<*mut better_siginfo_t, *mut libc::siginfo_t>(&mut info as *mut better_siginfo_t) };

            loop {
                let mut timeout = libc::timespec {
                    tv_sec: 1,
                    tv_nsec: 0
                };

                loop {

                    // Safe because we know that the first two pointers are valid,
                    // and the third argument can safely be NULL
                    let res = unsafe { libc::sigtimedwait(sigset_ptr, info_ptr, &mut timeout as *mut libc::timespec) };
                    if res == -1 {
                        if Errno::last() == Errno::EAGAIN {
                            continue;
                        }
                        println!("Error calling sigtimedwait: {}", nix::errno::errno());
                        continue;
                    }

                    {
                        // Critical section - we repeatedly call waitpid()
                        // to reap all children that have exited since the last
                        // signal
                        // We call waitpid with WNOHANG, which ensures
                        // that we don't block with the lock held
                        let proc_chans = &mut waiter_lock.lock().unwrap().proc_chans;


                        loop {
                            let res = waitpid(None, Some(WaitPidFlag::WNOHANG));
                            trace!("Waitpid result: {:?}", res);

                            if res.is_err() {
                                if res == Err(nix::Error::Sys(Errno::ECHILD)) {
                                    break;
                                }
                                panic!("Waitpid error: {:?}", res);
                            }
                            let res = res.ok().unwrap();

                            if res == WaitStatus::StillAlive {
                                break;
                            }

                            let pid = res.pid().unwrap();

                            let data = SignalData {
                                status: res,
                                pid
                            };

                            let sender: &Sender<SignalData> = &proc_chans.entry(pid)
                                .or_insert_with(ChanPair::new)
                                .sender;


                            sender.send(data).expect("Failed to send SignalData!");

                        }
                    }
                }
            }
        });
    }
}


#[derive(Debug)]
pub struct Process {
    binary: Binary,
    cmd: Command,
    child: Option<Child>,
    input: Vec<u8>,
    ptrace: bool
}

pub struct ProcessHandle {
    pid: Pid,
    inner: Arc<Mutex<ProcessWaiterInner>>,
    recv: Receiver<SignalData>,
    proc: Process
}

impl ProcessHandle {
    pub fn finish(&self, timeout: Duration) -> Result<Pid, SolverError> {
        let start = Instant::now();
        let mut time_left = timeout;

        loop {
            let data = self.recv.recv_timeout(time_left).expect("Receieve error!");
            match data.status {
                WaitStatus::Exited(_, _) => {
                    // Remove process data from the map now that it has exited
                    self.inner.lock().unwrap().proc_chans.remove(&data.pid);
                    return Ok(data.pid)
                },
                _ => {

                    let now = Instant::now();
                    let elapsed = now - start;
                    if elapsed > timeout {
                        // TODO - kill process?
                        return Err(SolverError::new(Runner::Timeout, "child timeout"))
                    }
                    time_left = match time_left.checked_sub(elapsed) {
                        Some(t) => t,
                        None => return Err(SolverError::new(Runner::Timeout, "child timed out"))
                    };

                    if self.proc.ptrace {
                        ptrace::cont(self.pid, None)
                            .unwrap_or_else(|e| panic!("Failed to call ptrace::cont for pid {:?}: {:?}", self.pid, e))
                    }

                },
            }
        }
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }


    // read buf to process then close it
    pub fn read_stdout(&mut self, buf: &mut Vec<u8>) -> Result<usize, SolverError> {
        if self.proc.child.is_none() {
            return Err(SolverError::new(
                Runner::RunnerError,
                "child process not running",
            ));
        }
        let child = self.proc.child.as_mut().unwrap();
        match child.stdout.as_mut() {
            Some(stdout) => stdout.read_to_end(buf).map_err(Into::into),
            None => Err(Error::last_os_error().into()),
        }
    }

}

// Handle running a process
impl Process {
    pub fn new(path: &str) -> Process {
        Process {
            binary: Binary::new(path),
            cmd: Command::new(path),
            input: Vec::new(),
            child: None,
            ptrace: false
        }
    }

    pub fn input(&mut self, stdin: Vec<u8>) {
        self.input = stdin
    }

    pub fn child_id(&self) -> Result<u32, SolverError> {
        match &self.child {
            Some(a) => Ok(a.id()),
            None => Err(SolverError::new(Runner::IoError, "no child id")),
        }
    }

    pub fn write_input(&mut self) -> Result<(), SolverError> {
        self.write_stdin(&self.input.clone())
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

        if self.ptrace {
            // Copied from spawn_ptrace
            self.cmd.before_exec(|| {
                ptrace::traceme().expect("TRACEME failed!");
                Ok(())
            });
        }

        let child = self.cmd.spawn();

        // spawn process and wait after fork
        //let child = self.cmd.spawn_ptrace();
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
            Some(stdin) => stdin.write_all(buf).map_err(Into::into),
            None => Err(SolverError::new(Runner::IoError, "could not open stdin")),
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
        ptrace::cont(Pid::from_raw(child.id() as i32), None).map_err(Into::into)
    }

    pub fn with_ptrace(&mut self, ptrace: bool) {
        self.ptrace = ptrace;
    }

    pub fn spawn(self) -> ProcessHandle {
        WAITER.spawn_process(self)
    }
}
