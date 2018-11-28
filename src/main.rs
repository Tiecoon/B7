extern crate clap;
extern crate libc;
extern crate nix;
extern crate spawn_ptrace;
extern crate threadpool;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate termion;
extern crate tui;

use b7tui::Ui;
use clap::{App, Arg};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
extern crate tui_logger;

use std::sync::mpsc::channel;
use threadpool::ThreadPool;

pub mod binary;
pub mod bindings;

pub mod process;
use process::Process;

pub mod generators;
use generators::*;

pub mod b7777;

pub mod b7tui;

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
fn brute<
    G: Generate<I> + std::fmt::Display,
    I: 'static + std::fmt::Display + Clone + std::fmt::Debug + std::marker::Send + std::cmp::Ord,
    B: b7tui::Ui,
>(
    path: &str,
    repeat: u32,
    gen: &mut G,
    get_inst_count: fn(&str, &Input) -> i64,
    terminal: &mut B,
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
                let mut avg: f64 = 0.0;
                let mut count: f64 = 0.0;
                for _ in 0..repeat {
                    let inst_count = get_inst_count(&test, &inp);
                    avg += inst_count as f64;
                    count += 1.0;
                    trace!("inst_count: {:?}", inst_count);
                }
                avg /= count;
                let _ = tx.send((inp_pair.0, avg as i64));
            });
        }
        let mut min: u64 = std::i64::MAX as u64;
        for _ in 0..num_jobs {
            let tmp = rx.recv().unwrap();
            if (tmp.1 as u64) < min {
                min = tmp.1 as u64;
            }
            results.push(tmp);
        }
        results.sort();

        terminal.update(&results, &min);

        terminal.wait();

        let good_idx = find_outlier(results.as_slice());
        if !gen.update(&good_idx.0) {
            break;
        }
    }
}

fn main() {
    let matches = App::new("B7")
        .version("0.1.0")
        .arg(
            Arg::with_name("binary")
                .help("Binary to brute force input for")
                .index(1)
                .required(true),
        ).get_matches();

    let path = matches.value_of("binary").unwrap();

    let mut terminal = b7tui::Tui::new();

    let mut argcgen = ArgcGenerator::new(0, 51);
    brute(path, 1, &mut argcgen, get_inst_count_perf, &mut terminal);
    let argc = argcgen.get_length();
    let mut argvlengen = ArgvLenGenerator::new(argc, 0, 15);
    brute(path, 5, &mut argvlengen, get_inst_count_perf, &mut terminal);
    let argvlens = argvlengen.get_lengths();

    let mut argvgen = ArgvGenerator::new(argc, argvlens, 0x20, 0x7e);
    brute(path, 5, &mut argvgen, get_inst_count_perf, &mut terminal);
    //let argvlens = argvlengen.get_lengths();

    let mut lgen = StdinLenGenerator::new(0, 51);
    brute(path, 1, &mut lgen, get_inst_count_perf, &mut terminal);
    let stdinlen = lgen.get_length();

    // TODO: We should have a good way of configuring the range
    let mut gen = StdinCharGenerator::new(stdinlen, 0x20, 0x7e);
    brute(path, 1, &mut gen, get_inst_count_perf, &mut terminal);
    terminal.done();
}
