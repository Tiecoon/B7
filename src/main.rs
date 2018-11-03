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
use std::ffi::OsStr;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::process::Command;

pub mod binary;
use binary::Binary;

pub mod process;
use process::Process;

pub mod generators;
use generators::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

fn get_inst_count_perf(path: &str, inp: Input) -> i64 {
    // TODO: error checking...
    let mut proc = Process::new(path);
    for arg in inp.argv.iter() {
        proc.arg(OsStr::from_bytes(arg));
    }
    proc.start();
    proc.write_stdin(&inp.stdin);
    proc.close_stdin();
    proc.init_perf();
    proc.finish();
    let ret = match proc.get_inst_count() {
        Ok(x) => x,
        Err(_) => -1,
    };
    proc.close_perf();
    ret
}

fn find_outlier(counts: &Vec<i64>) -> usize {
    let mut max: i64 = -1;
    let mut max_idx: usize = 0;
    for (i, count) in counts.iter().enumerate() {
        if *count > max {
            max = *count;
            max_idx = i;
        }
    }
    max_idx
}

// can take out Debug trait later
fn brute<G: Generate<I> + std::fmt::Debug, I: std::fmt::Debug>(
    path: &str,
    gen: &mut G,
    get_inst_count: fn(&str, Input) -> i64,
) {
    loop {
        let mut ids: Vec<I> = Vec::new();
        let mut inst_counts: Vec<i64> = Vec::new();
        for inp_pair in gen.by_ref() {
            ids.push(inp_pair.0);
            let inp = inp_pair.1;

            let inst_count = get_inst_count(path, inp);
            println!("inst_count: {:?}", inst_count);
            inst_counts.push(inst_count);
        }
        let good_idx = find_outlier(&inst_counts);
        println!("good_idx: {:?}", good_idx);
        println!("{:?}", gen);
        if !gen.update(&ids[good_idx]) {
            break;
        }
    }
}

fn main() {
    let path = "./tests/wyvern";
    let mut lgen = StdinLenGenerator::new(0, 51);
    brute(path, &mut lgen, get_inst_count_perf);
    let stdinlen = 29; //lgen.get_length();
    println!("stdin length: {:?}", stdinlen);
    let mut gen = StdinCharGenerator::new(&stdinlen);
    brute(path, &mut gen, get_inst_count_perf);
    println!("gen: {:?}", gen);
    /*let mut proc = Process::new("/bin/ls");
    println!("proc: {:?}", proc);
    println!("args: {:?}", proc.args(&["ls", "-al"]));
    println!("start: {:?}", proc.start());
    println!("init_perf: {:?}", proc.init_perf());
    println!("finish: {:?}", proc.finish());
    let inst_count = proc.get_inst_count();
    println!("inst_count: {:?}", inst_count);*/
    //test();
}

fn proc_test() -> (i64) {
    let child = Command::new("/bin/true")
        .spawn_ptrace()
        .expect("process creation failed");
    extern "C" {
        fn get_perf_fd(input: pid_t) -> c_int;
    }
    let mut _test: c_int = 0;
    unsafe {
        _test = get_perf_fd(pid_t::from(child.id() as i32));
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
