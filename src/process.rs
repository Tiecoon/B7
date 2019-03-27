use crate::binary::Binary;
use nix::sys::ptrace;
use nix::sys::signal::{self, SigSet, Signal, SigmaskHow};
use nix::sys::wait::{waitpid, WaitStatus, WaitPidFlag};
use nix::unistd::Pid;
use spawn_ptrace::CommandPtraceSpawn;
use std::ffi::OsStr;
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::process::{Child, Command, Stdio};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, channel, Sender, Receiver};
use std::time::{Duration, Instant};
use libc::sigtimedwait;
use std::thread::{self, ThreadId};

use crate::siginfo::better_siginfo_t;

#[derive(Debug)]
pub struct SignalData {
    //pub info: better_siginfo_t,
    // parsed from 'info' filed
    //pub status: SignalStatus,
    pub status: WaitStatus,
    pub pid: Pid
}


#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum SignalStatus {
    Exited,

    Other
}

// From <siginfo.h>
const CLD_EXITED: libc::c_int = 1;
const CLD_KILLED: libc::c_int = 2;
const CLD_DUMPED: libc::c_int = 3;
const CLD_TRAPPED: libc::c_int  = 4;
const CLD_STOPPED: libc::c_int = 5;
const CLD_CONTINUED: libc::c_int = 6;



// All of the raw pointers in this type
// are filled in with address by Linux,
// to provide information about the signal.
// We never actually try to deference them


pub struct ProcessWaiter {
    started: bool,
    inner: Arc<Mutex<ProcessWaiterInner>>,
    initialized: Mutex<HashSet<ThreadId>>,
    num_threads: usize,
}

struct ProcessWaiterInner {
    //seen: HashMap<Pid, Vec<SignalData>>,
    //channels: HashMap<Process, (Sender<SignalData>, Option<Receiver<SignalData>>)>,
    channels: Vec<(Process, Sender<SignalData>)>,
    read_chan: (Sender<()>, Option<Receiver<()>>)
}

impl ProcessWaiter {
    pub fn new(num_threads: usize) -> ProcessWaiter {
        let chan = channel();
        ProcessWaiter {
            inner: Arc::new(Mutex::new(ProcessWaiterInner {
                //channels: HashMap::new(),
                channels: Vec::new(),
                read_chan: (chan.0, Some(chan.1))
                //read_chan: channel()
            })),
            started: false,
            initialized: Mutex::new(HashSet::new()),
            num_threads: num_threads,
        }
    }

    pub fn start_thread(&mut self) {
        if self.started {
            panic!("Already started waiter thread!");
        }
        self.started = true;
        let recv = self.inner.lock().unwrap().read_chan.1.take().unwrap();
        ProcessWaiter::spawn_waiting_thread(self.num_threads, recv, self.inner.clone());
    }

    pub fn block_signal(&self) {
        let mut mask = SigSet::empty();
        mask.add(Signal::SIGCHLD);

        signal::pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&mask), None);

    }

    // Block SIGCHLD for the calling thread
    // Records the initialization for the thread
    pub fn init_for_thread(&self) {
        self.block_signal();
        //println!("Init for thread!");

        self.initialized.lock().unwrap().insert(thread::current().id());
    }

    pub fn register_process(&self, process: Process) -> ProcessHandle  {
        //println!("Registering process");
        let mut start = false;
        let mut recv;
        {
            let mut waiter = self.inner.lock().unwrap();
            //waiter.processes.pushor_insert(ProcessWaiter::make_channel).1.as_ref().unwrap().clone()

            let chan = channel();

            waiter.channels.push((process, chan.0));
            println!("Curent: {}/{}", waiter.channels.len(), self.num_threads);
            if waiter.channels.len() == self.num_threads {
                waiter.read_chan.0.send(()).unwrap();
            }

            recv = chan.1
        }
        ProcessHandle { recv }
    }

    fn spawn_waiting_thread(num_threads: usize, read_chan: Receiver<()>, waiter_lock: Arc<Mutex<ProcessWaiterInner>>) {
        assert_eq!(std::mem::size_of::<libc::siginfo_t>(), std::mem::size_of::<better_siginfo_t>());
        std::thread::spawn(move || {

            let mut chld_mask = SigSet::empty();
            chld_mask.add(Signal::SIGCHLD);
            signal::pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&chld_mask), None).unwrap();

            let mut mask = SigSet::all();
            let mut info: better_siginfo_t = unsafe { std::mem::zeroed() };

            let sigset_ptr = mask.as_ref() as *const libc::sigset_t;
            // Safe because we defined better_siginfo_t, to be compatible with libc::siginfo_t
            let info_ptr = unsafe { std::mem::transmute::<*mut better_siginfo_t, *mut libc::siginfo_t>(&mut info as *mut better_siginfo_t) };

            loop {
                eprintln!("Waiting for notification...");
                read_chan.recv().unwrap();
                eprintln!("Starting!");


                let processes: Vec<(Process, Sender<SignalData>)> = waiter_lock.lock().unwrap().channels.drain(..).collect();

                let mut pids: HashMap<i32, Sender<SignalData>> = HashMap::new();

                for process in processes {
                    let mut proc = process.0;
                    proc.start().unwrap();
                    proc.write_input().unwrap();
                    proc.close_stdin().unwrap();

                    pids.insert(proc.child_id().unwrap() as i32, process.1);

                    ptrace::cont(Pid::from_raw(proc.child_id().unwrap() as i32), None)
                        .unwrap_or_else(|e| panic!("Failed to call initial ptrace::cont for pid {:?}: {:?}", proc.child_id().unwrap(), e));


                    println!("Spawning: {:?} {:?}", proc, proc.child_id().unwrap());
                }

                println!("Pids: {:?}", pids);

                let mut timeout = libc::timespec {
                    tv_sec: 2,
                    tv_nsec: 0
                };

                /*println!("Current threads:");
                let tree = Command::new("pstree").args(&[std::process::id().to_string()])
                    .output()
                    .unwrap();

                println!("{}", String::from_utf8(tree.stdout).unwrap());
*/

                loop {

                    eprintln!("Waiting for signal...");
                    // Safe because we know that the first two pointers are valid,
                    // and the third argument can safely be NULL
                    let res = unsafe { libc::sigtimedwait(sigset_ptr, info_ptr, &mut timeout as *mut libc::timespec) };
                    eprintln!("GOT SIGNAL! {:?} si_code={:?}", res, unsafe { info.fields.si_code });
                    if (res == -1) {
                        println!("Error calling sigtimedwait: {}", nix::errno::errno());
                        continue;
                    }

                    loop {
                        let res = waitpid(None, Some(WaitPidFlag::WNOHANG));
                        println!("Waitpid result: {:?}", res);

                        if res.is_err() {
                            panic!("Waitpid error: {:?}", res);
                        }
                        let res = res.ok().unwrap();

                        if res == WaitStatus::StillAlive {
                            break;
                        }

                        let pid = res.pid().unwrap();
                        let pid_raw = pid.as_raw();

                        match res {
                            WaitStatus::Exited(_, _) => {},
                            _ => {
                                ptrace::cont(pid, None).unwrap_or_else(|e| panic!("Failed to call ptrace::cont for pid {:?}: {:?}", pid, e));
                            }
                        }


                        /*let (pid, status) = match res {
                            Ok(WaitStatus::Exited(pid, _)) => (pid, SignalStatus::Exited),
                            Ok(WaitStatus::StillAlive) => break, // All pending children events were consumed
                            Ok(_) => {

                                                                SignalStatus::Other
                            },
                            Err(e) => panic!("Error waiting for process: {:?}", e)

                        };*/

                        let data = SignalData {
                            status: res,
                            pid: pid
                        };

                        println!("Data: {:?}", data);


                        //println!("Status: {:?} Code: {:?} Pid: {:?}", status, unsafe { info.fields.si_code }, pid);


                        pids.get(&pid_raw).expect(&format!("Unknown pid {:?}", pid)).send(data).unwrap();

                        /*if res.is_err() || res == Ok(WaitStatus::StillAlive) {
                            break;
                        }*/
                    }

                    // Safe bcause si_signo is always safe to access
                    let status = match unsafe { info.fields.si_signo } {
                        libc::SIGCHLD => {
                            // Safe because si_code is always safe to access
                            match unsafe { info.fields.si_code } {
                                CLD_EXITED | CLD_KILLED | CLD_DUMPED => {
                                    SignalStatus::Exited
                                },
                                _ => SignalStatus::Other
                            }
                        },
                        _ => SignalStatus::Other
                    };


                    // Safe because this union field is always safe to access
                    let pid_raw = unsafe { info.fields.inner.kill.si_pid  };
                    let pid = Pid::from_raw(pid_raw);


                    
                    info = unsafe { std::mem::zeroed() };
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
    input: Vec<u8>
}

pub struct ProcessHandle {
    recv: Receiver<SignalData>
}

impl ProcessHandle {
    pub fn finish(&self, timeout: Duration) -> std::result::Result<Pid, String> {
        let mut start = Instant::now();
        let mut time_left = timeout;
        loop {
            let res = self.recv.recv_timeout(time_left);
            if res.is_err() {
                return Err(format!("Receive error: {:?}", res.err().unwrap()));
            }
            let data = res.ok().unwrap();
            match data.status {
                WaitStatus::Exited(_, _) => return Ok(data.pid),
                _ => {
                    let now = Instant::now();
                    let elapsed = now - start;
                    if elapsed > timeout {
                        return Err(format!("Timout: elapsed!"))
                    }
                    time_left = match time_left.checked_sub(elapsed) {
                        Some(t) => t,
                        None => return Err(format!("Timeout: no time left"))
                    };


                },
                //SignalStatus::Exited => return Ok(data.pid)
            }
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
        }
    }

    pub fn input(&mut self, stdin: Vec<u8>) {
        self.input = stdin
    }

    pub fn child_id(&self) -> Option<u32> {
        match &self.child {
            Some(a) => Some(a.id()),
            None => None,
        }
    }

    pub fn write_input(&mut self) -> Result<()> {
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
    pub fn start(&mut self) -> Result<()> {
        if self.child.is_some() {
            return Err(Error::new(
                ErrorKind::Other,
                "child process already running",
            ));
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
            Err(c) => Err(c),
        }
    }

    // write buf to process then close it
    pub fn write_stdin(&mut self, buf: &[u8]) -> Result<()> {
        if self.child.is_none() {
            return Err(Error::new(ErrorKind::Other, "child process not running"));
        }
        let child = self.child.as_mut().unwrap();
        match child.stdin.as_mut() {
            Some(stdin) => stdin.write_all(buf),
            None => Err(Error::last_os_error()),
        }
    }

    // read buf to process then close it
    pub fn read_stdout(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        if self.child.is_none() {
            return Err(Error::new(ErrorKind::Other, "child process not running"));
        }
        let child = self.child.as_mut().unwrap();
        match child.stdout.as_mut() {
            Some(stdout) => stdout.read_to_end(buf),
            None => Err(Error::last_os_error()),
        }
    }

    // close stdin to prevent any reads hanging
    pub fn close_stdin(&mut self) -> Result<()> {
        if self.child.is_none() {
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

    // continue executing ptrace if it is paused
    pub fn cont(&self) -> Result<()> {
        if self.child.is_none() {
            return Err(Error::new(ErrorKind::Other, "child process not running"));
        }
        let child = self.child.as_ref().unwrap();
        let res = ptrace::cont(Pid::from_raw(child.id() as i32), None);
        match res {
            Ok(_) => Ok(()),
            Err(x) => Err(Error::new(ErrorKind::Other, format!("{:?}", x))),
        }
    }

    // go until next pause point
    pub fn wait(&self, timout: Duration, waiter: &ProcessWaiter) -> Result<WaitStatus> {
        if self.child.is_none() {
            return Err(Error::new(ErrorKind::Other, "child process not running"));
        }
        let child = self.child.as_ref().unwrap();

        // We wait for a SIGCHLD using sigtimedwait
        // Based on https://www.linuxprogrammingblog.com/code-examples/signal-waiting-sigtimedwait
        unimplemented!();
    }

    // attempt to run the program to completion
    pub fn finish(&mut self, timeout: Duration, receiver: Receiver<SignalData>) -> Result<()> {
        //let receiver = waiter.register_pid(Pid::from_raw(self.child.as_ref().unwrap().id() as i32));

        loop {
            /*let cret = self.cont();
            if cret.is_err() {
                return cret;
            }*/
            match receiver.recv_timeout(Duration::new(5, 0)) {
                Ok(data) => {
                    println!("Got data :{:?}", data.status);
                    return Ok(());
                }
                Err(x) => {
                    println!("Stdout after timeout...");
                    let mut stdout = Vec::new();
                    self.read_stdout(&mut stdout);
                    println!("Stdout: {:?}", String::from_utf8(stdout).unwrap());
                    panic!("Timeout in wait!");
                }
                _ => (),
            }
        }
    }
}
