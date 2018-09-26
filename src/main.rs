extern crate libc;
extern crate nix;

use nix::sys::{ptrace, wait};
use nix::unistd::{execve, fork, ForkResult, Pid};
use std::ffi::CString;

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
    println!("PARENT: SLEEEPING\n\n\n");
    std::thread::sleep(std::time::Duration::new(5, 0));
    println!("PARENT: PTRACE CONTINUE\n\n\n");
    ptrace::cont(child, None).expect("attach fail");
    println!("PARENT: waitpid done");
}
