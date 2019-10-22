use crate::binary::Binary;
use crate::errors::Runner::ProcfsError;
use crate::errors::*;
use crate::generators::MemInput;
use byteorder::ByteOrder;
use lazy_static::lazy_static;
use nix::errno::Errno;
use nix::sys::ptrace;
use nix::sys::signal::{self, SigSet, SigmaskHow, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::convert::Into;
use std::ffi::OsStr;
use std::io::{Error, Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const WORD_SIZE: usize = std::mem::size_of::<usize>();

/// Map between breakpoint addresses and breakpoint information
type BreakpointMap = HashMap<usize, BreakpointInfo>;

// Represents data returned from a call to waitpid()
// For convenience, we include the pid directly in the
// struct, to avoid needing to unwrap it from WaitStatus
// repeatedly
#[derive(Debug)]
struct WaitData {
    pub status: WaitStatus,
    pub pid: Pid,
}

lazy_static! {
    /// The global ProcessWaiter instance
    /// This takes control of SIGCHLD handling for the entire
    /// process. For this reason, there can never be more than one,
    /// as they would interfere with each other.
    ///
    /// See [ProcessWaiter::spawn_process] for details on how to use it
    pub static ref WAITER: ProcessWaiter = { ProcessWaiter::new() };
}
/// ProcessWaiter allows waiting on child processes
/// while specifying a timeout. There is exactly
/// one instance of this struct for the entire process -
/// it's stored in [WAITER]
pub struct ProcessWaiter {
    started: bool,
    inner: Arc<Mutex<ProcessWaiterInner>>,
}

/// The Mutex-protected interior of a ProcessWaiter.
/// This is used to give the waiter thread access
/// to the part of ProcessWaiter that it actually uses,
/// avoiding the need to wrap the entire ProcessWaiter
/// in a mutex
struct ProcessWaiterInner {
    proc_chans: HashMap<Pid, ChanPair>,
}

/// Represents the two ends of an MPSC channel
/// The 'receiver' field will be taken by
/// the consumer (i.e. the caller of ProcessWaiter::spawn_process)
struct ChanPair {
    sender: Sender<WaitData>,
    receiver: Option<Receiver<WaitData>>,
}

impl ChanPair {
    fn new() -> ChanPair {
        let (sender, receiver) = channel();
        ChanPair {
            sender,
            receiver: Some(receiver),
        }
    }

    fn take_recv(&mut self) -> Receiver<WaitData> {
        self.receiver.take().expect("Already took receiver!")
    }
}

/// Blocks SIGCHLD for the current thread.
/// Normally, there's no need to call this function - ProcessWaiter
/// will automatically call it the first time it is used.
///
/// However, this function *must* be used when using ProcessWaiter
/// with the standard Rust testing framework (e.g. `#[test]` functions)
///
/// Because tests are run on separate threads, the main thread will
/// never have SIGCHLD blocked. This will prevent ProcessWaiter from
/// working properly, as SIGCHLD must be blocked on every thread.
///
/// In a testing environment, 'block_signal' must be somehow called
/// on the main thread. One approach is to use the `ctor` crate,
/// and register a contructor that calls `block_signal`.
///
/// For an example of what this looks like, see 'tests/run_wyvern.rs'
pub fn block_signal() {
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGCHLD);

    signal::pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&mask), None)
        .expect("Failed to block signals!");
}

impl ProcessWaiter {
    fn new() -> ProcessWaiter {
        let mut waiter = ProcessWaiter {
            inner: Arc::new(Mutex::new(ProcessWaiterInner {
                proc_chans: HashMap::new(),
            })),
            started: false,
        };
        block_signal();
        waiter.start_thread();
        waiter
    }

    fn start_thread(&mut self) {
        if self.started {
            panic!("Already started waiter thread!");
        }
        self.started = true;
        ProcessWaiter::spawn_waiting_thread(self.inner.clone());
    }

    // Block SIGCHLD for the calling thread
    // Records the initialization for the thread
    pub fn init_for_thread(&self) {
        block_signal();
    }

    /// Spawns a process, returing a ProcessHandle which can be
    /// used to interact with the spawned process.
    pub fn spawn_process(&self, mut process: Process) -> ProcessHandle {
        let recv;
        process.start().expect("Failed to spawn process!");
        process.write_input().unwrap();
        process.close_stdin().unwrap();

        let pid = Pid::from_raw(process.child_id().unwrap() as i32);

        {
            // Critical section - create channel pair if it does
            // not exist, and take the receiver end
            let proc_chans = &mut self.inner.lock().unwrap().proc_chans;

            recv = proc_chans
                .entry(pid)
                .or_insert_with(ChanPair::new)
                .take_recv();
        }
        ProcessHandle {
            pid,
            recv,
            inner: self.inner.clone(),
            proc: process,
        }
    }

    /// The core logic of ProcessWaiter. This is fairly tricky, due to the complications
    /// of Linux signal handling. It works like this:
    ///
    /// We call 'sigtimedwait' in a loop, with a signal mask containing only 'SIGCHLD'.
    /// Whenever we receieve a signal (which is guaranteed to be SIGCHLD),
    /// we call waitpid() in a loop with WNOHANG. This ensures that we process
    /// all child updates that have occured since our last call to 'sigtimedwait'.
    /// Due to how Linux signal delivery works, we are not guaranteed to receive
    /// a SIGCHLD for every single child event - if a SIGCHLD arives
    /// while another SIGCHLD is still pending, it won't be delievered.
    /// We then send the 'waitpid' result over an MPSC channel, where it
    /// will be consumed by the thread waiting on the child.
    ///
    /// There are a number of subtleties here:
    ///
    /// By 'waiter thead', we mean the thread spawned by this function.
    /// By 'spawned thread', we mean the thread that actually spawns
    /// a child process, via calling ProcessWaiter::spawn_process
    ///
    /// 1. We block SIGCHLD on every thread. Normally, ProcessWaiter
    /// will be initialized from the main thread. Since threads
    /// inherit the blocked signal set of their parent, this ensures
    /// that every thread has SIGCHLD blocked (unless a thread manually unblocks it).
    ///
    /// As described in sigtimedwait(2) [https://linux.die.net/man/2/sigtimedwait],
    /// and signal(7) [http://man7.org/linux/man-pages/man7/signal.7.html],
    /// deterministic handling a signal in a multi-threaded environment
    /// requires that the signal in question be unblocked on at most one thread.
    /// If multiple threads have a signal unblocked, the kernel chooses an
    /// arbitrary thread to deliver the signal to.
    ///
    /// In our case, we block SIGCHLD on *all* threads. This ensure
    /// that our call to `sigtimedwait` will receieve the SIGCHLD - otherwise,
    /// it could be delivered to some other thread.
    ///
    /// 2. When a consumer of ProcessWaiter wants to spawn a process,
    /// it calls 'spawn_process'. 'spawn_process' registers interest
    /// in the process by storing a new MPSC channel into the 'proc_chans'
    /// map, using the process PID as the key.
    ///
    /// However, since we use the PID as the key, it's only possible
    /// for the parent to update the map *after* the process has been spawned.
    /// This creates the potential for a race condition - if the process runs
    /// for a very short amount of time, it might exit before
    /// the parent has a chance to store the channel in the map.
    ///
    /// To avoid this race condition, we allow the waiter thread to *also*
    /// store the channel in the map. This creates two possible cases:
    ///
    /// Case 1: The spawned process lives long enough for the parent
    /// thread to store its PID and channel in the map. When it eventually
    /// exits, the waiter thread sees the existing channel, and sends
    /// the waitpid() data to the parent listening on the receive end of the channel.
    ///
    /// Case 2: The spawned process lives for a very short time. Specifically,
    /// the waiter thread receives a SIGCHLD before the spawner thread has a
    /// chance to update the map. In this case, the waiter thread will
    /// create a new channel, and send the waitpid data to the 'Sender'
    /// half of the channel. Because MPSC channels are buffered,
    /// the WaitData will simply remain in the queue until
    /// the spawner thread retrieves the 'Reciever' half of the channel from the map.
    fn spawn_waiting_thread(waiter_lock: Arc<Mutex<ProcessWaiterInner>>) {
        std::thread::spawn(move || {
            // Block SIGCHLD on this thread, just to be safe (in case
            // it somehow wasn't blocked on the parent thread)
            block_signal();

            let mut mask = SigSet::empty();
            mask.add(Signal::SIGCHLD);
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };

            let sigset_ptr = mask.as_ref() as *const libc::sigset_t;
            let info_ptr = &mut info as *mut libc::siginfo_t;

            loop {
                let mut timeout = libc::timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                };

                loop {
                    // Safe because we know that the first two pointers are valid,
                    // and the third argument can safely be NULL
                    let res = unsafe {
                        libc::sigtimedwait(
                            sigset_ptr,
                            info_ptr,
                            &mut timeout as *mut libc::timespec,
                        )
                    };
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

                            let data = WaitData { status: res, pid };

                            let sender: &Sender<WaitData> =
                                &proc_chans.entry(pid).or_insert_with(ChanPair::new).sender;

                            sender.send(data).expect("Failed to send WaitData!");
                        }
                    }
                }
            }
        });
    }
}

#[derive(Debug)]
struct BreakpointInfo {
    /// Creating a breakpoint requires injecting the breakpoint opcode into the
    /// process's code. The bytes that were overwritten for a breakpoint are
    /// saved here so they can be restored when the breakpoint is reached.
    saved_bytes: usize,
    /// Memory input associated with breakpoint
    mem_input: MemInput,
}

#[derive(Debug)]
pub struct Process {
    binary: Binary,
    cmd: Command,
    child: Option<Child>,
    stdin_input: Vec<u8>,
    mem_input: Vec<MemInput>,
    breakpoints: BreakpointMap,
    ptrace: bool,
}

pub struct ProcessHandle {
    pid: Pid,
    inner: Arc<Mutex<ProcessWaiterInner>>,
    recv: Receiver<WaitData>,
    proc: Process,
}

impl ProcessHandle {
    /// Get the process's base address from /proc/<pid>/maps
    fn get_base_addr(&self) -> SolverResult<usize> {
        let proc = procfs::Process::new(self.pid.as_raw())?;
        let maps = proc.maps()?;
        let exe_path = proc.exe()?;
        let base_map = maps
            .iter()
            .filter(|map| match map.pathname {
                procfs::MMapPath::Path(ref path) => path == &exe_path,
                _ => false,
            })
            .next()
            .ok_or_else(|| {
                SolverError::new(
                    ProcfsError,
                    "Failed to get proc base address while writing memory input",
                )
            })?;

        Ok(base_map.address.0 as usize)
    }

    /// Write each memory input range to the process
    /// NOTE: This assumes `self.proc.ptrace` is `true`
    fn write_mem_input(&self, mem: &MemInput) -> SolverResult<()> {
        let is_pie = self.proc.binary.is_pie()?;

        for (nth_word, word) in mem.bytes.chunks(WORD_SIZE).enumerate() {
            // Use relative address if binary is PIE
            let addr = if is_pie {
                mem.addr + self.get_base_addr()?
            } else {
                mem.addr
            };
            let addr = addr + nth_word * WORD_SIZE;
            let addr = addr as ptrace::AddressType;

            // Pad to word size
            let word = {
                let mut word = word.to_vec();
                word.resize(WORD_SIZE, 0x00);
                word
            };

            // Convert from bytes to word
            let word = byteorder::NativeEndian::read_uint(&word, WORD_SIZE);
            let word = word as ptrace::AddressType;

            // Do the write
            ptrace::write(self.pid, addr, word)?;
        }

        Ok(())
    }

    fn add_breakpoint(&self, addr: usize, mem_input: &MemInput) -> SolverResult<BreakpointInfo> {
        let bytes = ptrace::read(self.pid, addr as ptrace::AddressType)? as usize;

        // 0xcc is the x86 int3 breakpoint opcode. We can assume little endian
        // here, since breakpoints are only supported on x86.
        let bp_bytes = bytes & (std::usize::MAX ^ 0xff) | 0xcc;
        ptrace::write(
            self.pid,
            addr as ptrace::AddressType,
            bp_bytes as ptrace::AddressType,
        )?;

        Ok(BreakpointInfo {
            saved_bytes: bytes,
            mem_input: mem_input.clone(),
        })
    }

    fn init_mem_input(&self, breakpoints: &mut BreakpointMap) -> SolverResult<()> {
        for mem in &self.proc.mem_input {
            match mem.breakpoint {
                Some(bp_addr) => {
                    let bp_info = self.add_breakpoint(bp_addr, mem)?;
                    breakpoints.insert(bp_addr, bp_info);
                }
                None => self.write_mem_input(mem)?,
            }
        }

        Ok(())
    }

    fn handle_ptrace_stop(
        &self,
        init_ptrace: &mut bool,
        breakpoints: &mut BreakpointMap,
    ) -> SolverResult<()> {
        if *init_ptrace {
            self.init_mem_input(breakpoints)?;
            *init_ptrace = false;
        }

        // Check if the instruction pointer is at a breakpoint
        let ip = ptrace::getregs(self.pid)?.rip as usize; // TODO: check for other archs
        if let Some(bp_info) = breakpoints.get(&ip) {
            self.write_mem_input(&bp_info.mem_input)?;

            // Remove breakpoint
            ptrace::write(
                self.pid,
                ip as ptrace::AddressType,
                bp_info.saved_bytes as ptrace::AddressType,
            )?;
        }

        // Continue process
        ptrace::cont(self.pid, None).unwrap_or_else(|e| {
            panic!(
                "Failed to call ptrace::cont for pid {:?}: {:?}",
                self.pid, e
            )
        });

        Ok(())
    }

    /// run process until it exits or times out
    pub fn finish(&self, timeout: Duration) -> SolverResult<Pid> {
        let start = Instant::now();
        let mut time_left = timeout;
        let mut init_ptrace = true;
        let mut breakpoints = BreakpointMap::new();

        loop {
            let data = self.recv.recv_timeout(time_left).expect("Receieve error!");
            match data.status {
                WaitStatus::Exited(_, _) => {
                    // Remove process data from the map now that it has exited
                    self.inner.lock().unwrap().proc_chans.remove(&data.pid);
                    return Ok(data.pid);
                }
                _ => {
                    let now = Instant::now();
                    let elapsed = now - start;
                    if elapsed > timeout {
                        // TODO - kill process?
                        return Err(SolverError::new(Runner::Timeout, "child timeout"));
                    }
                    time_left = match time_left.checked_sub(elapsed) {
                        Some(t) => t,
                        None => return Err(SolverError::new(Runner::Timeout, "child timed out")),
                    };

                    if self.proc.ptrace {
                        self.handle_ptrace_stop(&mut init_ptrace, &mut breakpoints);
                    }
                }
            }
        }
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// reads process stdout into buf and returns number of bytes read
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
    pub fn new(path: &str) -> SolverResult<Process> {
        Ok(Process {
            binary: Binary::new(path)?,
            cmd: Command::new(path),
            stdin_input: Vec::new(),
            mem_input: Vec::new(),
            child: None,
            breakpoints: HashMap::new(),
            ptrace: false,
        })
    }

    /// set what stdin should be sent to process
    pub fn stdin_input(&mut self, stdin: Vec<u8>) {
        self.stdin_input = stdin
    }

    /// set what memory input should be sent to process
    pub fn mem_input(&mut self, mem: Vec<MemInput>) {
        self.mem_input = mem
    }

    /// returns PID of child process
    pub fn child_id(&self) -> Result<u32, SolverError> {
        match &self.child {
            Some(a) => Ok(a.id()),
            None => Err(SolverError::new(Runner::IoError, "no child id")),
        }
    }

    /// writes self.stdin_input to the process's stdin
    pub fn write_input(&mut self) -> Result<(), SolverError> {
        self.write_stdin(&self.stdin_input.clone())
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

    /// initialize process according to settings
    pub fn start(&mut self) -> Result<(), SolverError> {
        if self.child.is_some() {
            return Err(SolverError::new(Runner::Unknown, "child already running"));
        }
        self.cmd.stdin(Stdio::piped());
        self.cmd.stdout(Stdio::piped());
        self.cmd.stderr(Stdio::piped());

        if self.ptrace {
            // Copied from spawn_ptrace
            unsafe {
                self.cmd.pre_exec(|| {
                    ptrace::traceme().expect("TRACEME failed!");
                    Ok(())
                });
            }
        }

        let child = self.cmd.spawn();

        // spawn process and wait after fork
        match child {
            Ok(c) => {
                self.child = Some(c);
                Ok(())
            }
            Err(x) => Err(x.into()),
        }
    }

    /// write buf to process stdin then close stdin
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

    /// close process stdin
    ///
    /// helps if child process is hanging on a read from stdin
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

    /// set wether or not the process should be run under ptrace
    pub fn with_ptrace(&mut self, ptrace: bool) {
        self.ptrace = ptrace;
    }

    /// spawn process
    pub fn spawn(self) -> ProcessHandle {
        WAITER.spawn_process(self)
    }
}
