// Needed for bindgen bindings
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

extern crate libc;
extern crate nix;

use libc::{c_int, c_void, pid_t};
use nix::sys::{ptrace, wait};
use nix::unistd::{execve, fork, ForkResult, Pid};
use std::ffi::CString;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

fn main() {
    match fork() {
        Ok(ForkResult::Parent { child, .. }) => parent(child),
        Ok(ForkResult::Child) => child(),
        Err(_) => println!("Fork failed"),
    }
}

fn child() {
    assert!(ptrace::traceme().is_ok());
    println!("CHILD: execve");
    execve(
        &CString::new("/bin/ls").expect("1"),
        &[CString::new("a").expect("2")],
        &[CString::new("a").expect("3")],
    ).expect("CHILD: execve failed");
    println!("CHILD: forking done");
}

fn parent(child: Pid) {
    println!(
        "Continuing execution in parent process, new child has pid: {}",
        child
    );
    println!("PARENT: pid, {}", std::process::id());
    wait::waitpid(child, Some(wait::WaitPidFlag::WSTOPPED)).expect("waitpid failed");
    // println!("PARENT: SLEEEPING\n\n\n");
    // std::thread::sleep(std::time::Duration::new(5, 0));

    // setup perf_event_open and return file descriptor to be read from
    extern "C" {
        fn b77(input: pid_t) -> c_int;
    }
    let mut _test: c_int = 0;
    unsafe {
        _test = b77(pid_t::from(child));
    }

    // continue execution
    println!("{}", _test);
    println!("PARENT: PTRACE CONTINUE\n");
    ptrace::cont(child, None).expect("ptrace fail");
    wait::wait().expect("HUH");

    // read in number of unstructions
    let mut count: i64 = 0; // long long
    let count_ptr: *mut c_void = &mut count as *mut _ as *mut c_void;
    unsafe {
        libc::read(_test, count_ptr, 8);
    }

    println!("PARENT: instructions: {}", count);
    println!("PARENT: waitpid done");
}
