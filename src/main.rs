extern crate clap;
extern crate libc;
extern crate nix;
extern crate spawn_ptrace;
extern crate threadpool;

#[macro_use]
extern crate log;
extern crate env_logger;

use clap::{App, Arg};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

use std::sync::mpsc::channel;
use threadpool::ThreadPool;

pub mod binary;
pub mod bindings;

pub mod process;
use process::Process;

pub mod generators;
use generators::*;

pub mod b7777;

fn get_inst_count_perf(path: &str, inp: &Input) -> i64 {
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

// Find the most distant point from the average.

// Returns (index, value) of this point. (TODO: fix this)
fn find_outlier<I: std::fmt::Debug>(counts: &[(I, i64)]) -> &(I, i64) {
    // Calculate the average
    let mut avg: i64 = 0;
    for (_, count) in counts.iter() {
        avg += count;
    }
    if !counts.is_empty() {
        avg /= counts.len() as i64;
    } else {
        // Handle division by zero
        warn!("WWWWWWWW {:?}", counts);
    }
    // and then find the most distant point
    let mut max_dist: i64 = -1;
    let mut max_idx: usize = 0;
    for (i, (_, count)) in counts.iter().enumerate() {
        let dist: i64 = (count - avg).abs();
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }
    //(max_idx, max_val)
    &counts[max_idx]
}

// can take out Debug trait later
fn brute<G: Generate<I> + std::fmt::Display, I: 'static + std::fmt::Debug + std::marker::Send>(
    path: &str,
    gen: &mut G,
    get_inst_count: fn(&str, &Input) -> i64,
) {
    loop {
        let n_workers = 8;
        let mut num_jobs: i64 = 0;
        let mut results: Vec<(I, i64)> = Vec::new();

        let pool = ThreadPool::new(n_workers);
        let (tx, rx) = channel();

        for inp_pair in gen.by_ref() {
            num_jobs += 1;
            let tx = tx.clone();
            let test = String::from(path);
            pool.execute(move || {
                let inp = inp_pair.1;
                let inst_count = get_inst_count(&test, &inp);
                trace!("inst_count: {:?}", inst_count);
                let _ = tx.send((inp_pair.0, inst_count));
            });
        }
        for _ in 0..num_jobs {
            results.push(rx.recv().unwrap());
        }
        let good_idx = find_outlier(&results);
        if !gen.update(&good_idx.0) {
            break;
        }
    }
}

fn main() {
    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    env_logger::Builder::from_env(env)
        .default_format_timestamp(false)
        .init();

    let matches = App::new("B7")
        .version("0.1.0")
        .arg(
            Arg::with_name("binary")
                .help("Binary to brute force input for")
                .index(1)
                .required(true),
        ).get_matches();

    let path = matches.value_of("binary").unwrap();
    let mut lgen = StdinLenGenerator::new(0, 51);
    brute(path, &mut lgen, get_inst_count_perf);
    let stdinlen = lgen.get_length();
    // TODO: We should have a good way of configuring the range
    let mut gen = StdinCharGenerator::new(stdinlen, 0x20, 0x7e);
    brute(path, &mut gen, get_inst_count_perf);
    info!("Successfully Generated: {}", gen);
}
