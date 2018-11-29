extern crate clap;
extern crate env_logger;
extern crate libc;
#[macro_use]
extern crate log;
extern crate nix;
extern crate regex;
extern crate spawn_ptrace;
extern crate termion;
extern crate threadpool;
extern crate tui;
extern crate tui_logger;

use b7tui::Ui;
use clap::{App, Arg};
use generators::*;
use std::fs::File;
use std::io::prelude::*;
use std::sync::mpsc::channel;
use threadpool::ThreadPool;

pub mod b7tui;
pub mod binary;
pub mod bindings;
pub mod dynamorio;
pub mod generators;
pub mod perf;
pub mod process;
pub mod statistics;

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
        let good_idx = statistics::find_outlier(results.as_slice());
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
        ).arg(
            Arg::with_name("solver")
                .short("s")
                .long("solver")
                .value_name("solver")
                .help("Sets which solver to use (default perf)")
                .takes_value(true),
        ).arg(
            Arg::with_name("ui")
                .short("u")
                .long("ui")
                .value_name("ui_type")
                .help("Sets which interface to use (default Tui)")
                .takes_value(true),
        ).get_matches();

    let path = matches.value_of("binary").unwrap();

    let solvername = matches.value_of("solver").unwrap_or("perf");
    let solver: fn(&str, &Input) -> i64;

    match solvername {
        "perf" => solver = perf::get_inst_count,
        "dynamorio" => solver = dynamorio::get_inst_count,
        _ => panic!("unknown solver"),
    }

    let mut terminal = b7tui::Tui::new();
    info!("using {}", solvername);

    // Solve for argc
    let mut argcgen = ArgcGenerator::new(0, 5);
    brute(path, 1, &mut argcgen, solver, &mut terminal);
    let argc = argcgen.get_length();

    let mut file = File::create(format!("{}.cache", path)).unwrap();
    // check if there is something to be solved
    if argc > 0 {
        // solve argv length
        let mut argvlengen = ArgvLenGenerator::new(argc, 0, 20);
        brute(path, 5, &mut argvlengen, solver, &mut terminal);
        let argvlens = argvlengen.get_lengths();

        // solve argv values
        let mut argvgen = ArgvGenerator::new(argc, argvlens, 0x20, 0x7e);
        brute(path, 5, &mut argvgen, solver, &mut terminal);
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
    brute(path, 1, &mut lgen, solver, &mut terminal);
    let stdinlen = lgen.get_length();
    //solve strin if there is stuff to solve
    if stdinlen > 0 {
        // TODO: We should have a good way of configuring the range
        let mut gen = StdinCharGenerator::new(stdinlen, 0x20, 0x7e);
        brute(path, 1, &mut gen, solver, &mut terminal);
        let std = gen.get_input().clone();
        file.write_all(String::from_utf8_lossy(std.as_slice()).as_bytes())
            .unwrap();
    }

    // let terminal decide if it should wait for user
    terminal.done();
}
