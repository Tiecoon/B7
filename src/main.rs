extern crate clap;
extern crate env_logger;
extern crate libc;
#[macro_use]
extern crate log;
extern crate nix;
extern crate spawn_ptrace;
extern crate termion;
extern crate threadpool;
extern crate tui;
extern crate tui_logger;

use b7tui::Ui;
use clap::{App, Arg};
use generators::*;
use process::Process;
use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::os::unix::ffi::OsStrExt;
use std::sync::mpsc::channel;
use threadpool::ThreadPool;

pub mod b7777;
pub mod b7tui;
pub mod binary;
pub mod bindings;
pub mod generators;
pub mod process;

// Handles basic proc spawning and running under perf
fn get_inst_count_perf(path: &str, inp: &Input) -> i64 {
    // TODO: error checking...
    let mut proc = Process::new(path);
    for arg in inp.argv.iter() {
        proc.arg(OsStr::from_bytes(arg));
    }

    // Start Process run it to completion with all arguements
    proc.start().unwrap();
    proc.write_stdin(&inp.stdin).unwrap();
    proc.close_stdin().unwrap();
    proc.init_perf().unwrap();
    proc.finish().unwrap();

    // Process instruction count
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

    &counts[max_idx]
}

// can take out Debug trait later
// Combines the generators with the instruction counters to deduce the next step
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
    // Loop until generator says we are done
    loop {
        // Number of threads to spawn
        let n_workers = 8;
        let mut num_jobs: i64 = 0;
        let mut results: Vec<(I, i64)> = Vec::new();

        let pool = ThreadPool::new(n_workers);
        let (tx, rx) = channel();

        // run each case of the generators
        for inp_pair in gen.by_ref() {
            num_jobs += 1;
            let tx = tx.clone();
            let test = String::from(path);
            // give it to a thread to handle
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
        // Track the minimum for stats later
        let mut min: u64 = std::i64::MAX as u64;
        // Get results from the threads
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

        // inform generator of the result
        let good_idx = find_outlier(results.as_slice());
        if !gen.update(&good_idx.0) {
            break;
        }
    }
}

fn main() {
    // handle command line arguements
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

    // Solve for argc
    let mut argcgen = ArgcGenerator::new(0, 5);
    brute(path, 1, &mut argcgen, get_inst_count_perf, &mut terminal);
    let argc = argcgen.get_length();

    let mut file = File::create(format!("{}.cache", path)).unwrap();
    // check if there is something to be solved
    if argc > 0 {
        // solve argv length
        let mut argvlengen = ArgvLenGenerator::new(argc, 0, 20);
        brute(path, 5, &mut argvlengen, get_inst_count_perf, &mut terminal);
        let argvlens = argvlengen.get_lengths();

        // solve argv values
        let mut argvgen = ArgvGenerator::new(argc, argvlens, 0x20, 0x7e);
        brute(path, 5, &mut argvgen, get_inst_count_perf, &mut terminal);
        let argv = argvgen.get_argv();
        // TODO: error handling could be improved here
        file.write_all(b"[").unwrap();
        for arg in argv {
            file.write_all(String::from_utf8_lossy(arg.as_slice()).as_bytes())
                .unwrap();
        }
        file.write_all(b"]\n").unwrap();
    }
    //solve stdin len
    let mut lgen = StdinLenGenerator::new(0, 51);
    brute(path, 1, &mut lgen, get_inst_count_perf, &mut terminal);
    let stdinlen = lgen.get_length();
    //solve strin if there is stuff to solve
    if stdinlen > 0 {
        // TODO: We should have a good way of configuring the range
        let mut gen = StdinCharGenerator::new(stdinlen, 0x20, 0x7e);
        brute(path, 1, &mut gen, get_inst_count_perf, &mut terminal);
        let std = gen.get_input().clone();
        file.write_all(String::from_utf8_lossy(std.as_slice()).as_bytes())
            .unwrap();
    }

    // let terminal decide if it should wait for user
    terminal.done();
}
