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
// Returns (index, value) of this point.
fn find_outlier(counts: &[i64]) -> usize {
    // Calculate the average
    let mut avg: i64 = 0;
    for count in counts {
        avg += count;
    }
    if counts.is_empty() {
        avg /= counts.len() as i64;
    } else {
        // Handle division by zero
        avg = 0;
    }
    // and then find the most distant point
    let mut max_dist: i64 = -1;
    let mut max_idx: usize = 0;
    for (i, count) in counts.iter().enumerate() {
        let dist: i64 = (*count - avg).abs();
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }
    //(max_idx, max_val)
    max_idx
}

// can take out Debug trait later
fn brute<G: Generate<I> + std::fmt::Display, I: std::fmt::Debug>(
    path: &str,
    gen: &mut G,
    get_inst_count: fn(&str, &Input) -> i64,
) {
    loop {
        let mut ids: Vec<I> = Vec::new();
        let mut inst_counts: Vec<i64> = Vec::new();
        for inp_pair in gen.by_ref() {
            ids.push(inp_pair.0);
            let inp = inp_pair.1;

            let inst_count = get_inst_count(path, &inp);
            inst_counts.push(inst_count);
        }
        let good_idx = find_outlier(&inst_counts);
        if !gen.update(&ids[good_idx]) {
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
