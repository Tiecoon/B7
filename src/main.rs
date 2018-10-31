// Needed for bindgen bindings
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

extern crate libc;
extern crate nix;
extern crate spawn_ptrace;
extern crate threadpool;

use libc::{c_int, c_void, pid_t};
use nix::sys::{ptrace, wait};
use nix::unistd::Pid;
use spawn_ptrace::CommandPtraceSpawn;
use std::process::Command;

pub mod binary;
use binary::Binary;

pub mod process;
use process::Process;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

fn main() {
    test();
}

fn proc_test() -> (i64) {
    let child = Command::new("/bin/true")
        .spawn_ptrace()
        .expect("process creation failed");
    extern "C" {
        fn b77(input: pid_t) -> c_int;
    }
    let mut _test: c_int = 0;
    unsafe {
        _test = b77(pid_t::from(child.id() as i32));
    }

    // continue execution
    println!("{}", _test);
    let PID = Pid::from_raw(pid_t::from(child.id() as i32));
    ptrace::cont(PID, None).expect("ptrace cont failed");

    let _ = wait::waitpid(PID, Some(wait::WaitPidFlag::empty()));

    // read in number of unstructions
    let mut count: i64 = 0; // long long
    let count_ptr: *mut c_void = &mut count as *mut _ as *mut c_void;
    unsafe {
        libc::read(_test, count_ptr, 8);
    }

    return count;
}

fn test() {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use threadpool::ThreadPool;
    let (tx, rx) = mpsc::channel();
    let n_jobs = 100;
    let n_workers = 20;
    let pool = ThreadPool::new(n_workers);
    for i in 0..n_jobs {
        let tx = tx.clone();
        println!("queuing {}", i);
        pool.execute(move || {
            println!("exec {}", i);
            let x = proc_test();
            tx.send((x, i))
                .expect("channel will be there waiting for the pool");
        });
    }

    thread::sleep(Duration::from_secs(1));
    for p in 0..n_jobs {
        let (j, o) = rx.recv().unwrap();
        println!("Got: {:#3} {:#3}/{:} {:#8} {:#30b}", o, p, n_jobs - 1, j, j);
    }
    println!("END");
}
