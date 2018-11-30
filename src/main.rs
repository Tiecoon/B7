extern crate clap;
extern crate env_logger;
extern crate libc;
#[macro_use]
extern crate log;
extern crate nix;
extern crate num_cpus;
extern crate regex;
extern crate spawn_ptrace;
extern crate termion;
extern crate threadpool;
extern crate tui;
extern crate tui_logger;

pub mod b7tui;
pub mod binary;
pub mod bindings;
pub mod brute;
pub mod dynamorio;
pub mod generators;
pub mod perf;
pub mod process;
pub mod statistics;

use b7tui::Ui;
use brute::brute;
use clap::{App, Arg};
use generators::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::process::exit;

fn handle_cli_args<'a>() -> clap::ArgMatches<'a> {
    App::new("B7")
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
        ).arg(
            Arg::with_name("start")
                .long("start")
                .value_name("String")
                .help("Start with a premade input")
                .takes_value(true),
        ).arg(
            Arg::with_name("argstate")
                .long("no-arg")
                .help("toggle running arg checks"),
        ).arg(
            Arg::with_name("stdinstate")
                .long("no-stdin")
                .help("toggle running stdin checks"),
        ).arg(
            Arg::with_name("dynpath")
                .long("dynpath")
                .help("Path to DynamoRio"),
        ).get_matches()
}

fn print_usage(matches: &clap::ArgMatches) -> ! {
    println!("{}", matches.usage());
    exit(-1);
}

fn main() {
    // handle command line arguements
    let matches = handle_cli_args();

    let path = match matches.value_of("binary") {
        Some(a) => a,
        None => print_usage(&matches),
    };

    let argstate = matches.occurrences_of("argstate") < 1;
    let stdinstate = matches.occurrences_of("stdinstate") < 1;

    let solvername = matches.value_of("solver").unwrap_or("perf");
    let solver = match solvername {
        "perf" => perf::get_inst_count,
        "dynamorio" => dynamorio::get_inst_count,
        _ => panic!("unknown solver"),
    };

    let stdin_input = matches.value_of("start").unwrap_or("");
    let mut vars = HashMap::new();
    let dynpath = matches.value_of("dynpath").unwrap_or("");
    vars.insert(String::from("dynpath"), String::from(dynpath));

    let mut terminal = b7tui::Tui::new();
    info!("Using {} solver", solvername);

    let mut file = File::create(format!("{}.cache", path)).unwrap();
    if argstate {
        // Solve for argc
        let mut argcgen = ArgcGenerator::new(0, 5);
        brute(path, 1, &mut argcgen, solver, &mut terminal, vars.clone());
        let argc = argcgen.get_length();

        // check if there is something to be solved
        if argc > 0 {
            // solve argv length
            let mut argvlengen = ArgvLenGenerator::new(argc, 0, 20);
            brute(
                path,
                5,
                &mut argvlengen,
                solver,
                &mut terminal,
                vars.clone(),
            );
            let argvlens = argvlengen.get_lengths();

            // solve argv values
            let mut argvgen = ArgvGenerator::new(argc, argvlens, 0x20, 0x7e);
            brute(path, 5, &mut argvgen, solver, &mut terminal, vars.clone());
            let argv = argvgen.get_argv();
            // TODO: error handling could be improved here
            file.write_all(b"[").unwrap();
            for arg in argv {
                file.write_all(String::from_utf8_lossy(arg.as_slice()).as_bytes())
                    .unwrap();
            }
            file.write_all(b"]\n").unwrap();
        }
    }
    if stdinstate {
        // solve stdin len
        let mut lgen = StdinLenGenerator::new(0, 51);
        brute(path, 1, &mut lgen, solver, &mut terminal, vars.clone());
        let stdinlen = lgen.get_length();
        // solve strin if there is stuff to solve
        if stdinlen > 0 {
            // TODO: We should have a good way of configuring the range
            let mut gen = if stdin_input == "" {
                StdinCharGenerator::new(stdinlen, 0x20, 0x7e)
            } else {
                StdinCharGenerator::new_start(stdinlen, 0x20, 0x7e, stdin_input.as_bytes())
            };
            brute(path, 1, &mut gen, solver, &mut terminal, vars.clone());
            let std = gen.get_input().clone();
            file.write_all(String::from_utf8_lossy(std.as_slice()).as_bytes())
                .unwrap();
        }
    }

    // let terminal decide if it should wait for user
    terminal.done();
}
