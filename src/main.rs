extern crate libc;
extern crate nix;
extern crate spawn_ptrace;
extern crate threadpool;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

pub mod binary;

pub mod process;
use process::Process;

pub mod generators;
use generators::*;

fn get_inst_count_perf(path: &str, inp: Input) -> i64 {
    // TODO: error checking...
    let mut proc = Process::new(path);
    for arg in inp.argv.iter() {
        proc.arg(OsStr::from_bytes(arg));
    }
    proc.start().unwrap();
    proc.write_stdin(&inp.stdin).unwrap();
    proc.close_stdin().unwrap();
    proc.init_perf().unwrap();
    proc.finish().unwrap();
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
    println!("gen: {}", gen);
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
