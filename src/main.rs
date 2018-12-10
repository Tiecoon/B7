#[macro_use]
extern crate log;

pub mod b7tui;
pub mod binary;
pub mod bindings;
pub mod brute;
pub mod dynamorio;
pub mod generators;
pub mod perf;
pub mod process;
pub mod statistics;

use crate::brute::brute;
use crate::generators::*;
use clap::{App, Arg};
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
        )
        .arg(
            Arg::with_name("solver")
                .short("s")
                .long("solver")
                .value_name("solver")
                .help("Sets which solver to use (default perf)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ui")
                .short("u")
                .long("ui")
                .value_name("ui_type")
                .help("Sets which interface to use (default Tui)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("start")
                .long("start")
                .value_name("String")
                .help("Start with a premade input")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("argstate")
                .long("no-arg")
                .help("toggle running arg checks"),
        )
        .arg(
            Arg::with_name("stdinstate")
                .long("no-stdin")
                .help("toggle running stdin checks"),
        )
        .arg(
            Arg::with_name("dynpath")
                .long("dynpath")
                .help("Path to DynamoRio build folder")
                .takes_value(true),
        )
        .get_matches()
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
    vars.insert(String::from("stdininput"), String::from(stdin_input));

    let terminal = String::from(matches.value_of("ui").unwrap_or("tui")).to_lowercase();

    let mut file = match File::open(format!("{}.cache", path)) {
        Ok(x) => x,
        _ => File::create(format!("{}.cache", path)).unwrap(),
    };

    // unsure on wether an enum would be better for readability but more conversion so..
    match &*terminal {
        "tui" => main2(
            path,
            argstate,
            stdinstate,
            &mut file,
            solver,
            &mut b7tui::Tui::new(Some(String::from(path))),
            vars,
        ),
        "env" => main2(
            path,
            argstate,
            stdinstate,
            &mut file,
            solver,
            &mut b7tui::Env::new(),
            vars,
        ),
        _ => panic!("unknown tui"),
    }
}

// transistion to handle terminal generic
fn main2<B: b7tui::Ui>(
    path: &str,
    argstate: bool,
    stdinstate: bool,
    file: &mut File,
    solver: fn(&str, &Input, &HashMap<String, String>) -> i64,
    terminal: &mut B,
    vars: HashMap<String, String>,
) {
    if argstate {
        default_arg_brute(path, solver, vars.clone(), terminal, file);
    }

    if stdinstate {
        default_stdin_brute(path, solver, vars.clone(), terminal, file);
    }

    // let terminal decide if it should wait for user
    terminal.done();
}

// solves "default" arguement case
fn default_arg_brute<B: b7tui::Ui>(
    path: &str,
    solver: fn(&str, &Input, &HashMap<String, String>) -> i64,
    vars: HashMap<String, String>,
    terminal: &mut B,
    file: &mut File,
) {
    // Solve for argc
    let mut argcgen = ArgcGenerator::new(0, 5);
    brute(path, 1, &mut argcgen, solver, terminal, vars.clone());
    let argc = argcgen.get_length();

    // check if there is something to be solved
    if argc > 0 {
        // solve argv length
        let mut argvlengen = ArgvLenGenerator::new(argc, 0, 20);
        brute(path, 5, &mut argvlengen, solver, terminal, vars.clone());
        let argvlens = argvlengen.get_lengths();

        // solve argv values
        let mut argvgen = ArgvGenerator::new(argc, argvlens, 0x20, 0x7e);
        brute(path, 5, &mut argvgen, solver, terminal, vars.clone());

        // TODO: error handling could be improved here
        let _ = file.write_fmt(format_args!("argv: {}", argvgen));
    }
}

// solves "default" stdin case
fn default_stdin_brute<B: b7tui::Ui>(
    path: &str,
    solver: fn(&str, &Input, &HashMap<String, String>) -> i64,
    vars: HashMap<String, String>,
    terminal: &mut B,
    file: &mut File,
) {
    // solve stdin len
    let mut lgen = StdinLenGenerator::new(0, 51);
    brute(path, 1, &mut lgen, solver, terminal, vars.clone());
    let stdinlen = lgen.get_length();
    // solve strin if there is stuff to solve
    if stdinlen > 0 {
        // TODO: We should have a good way of configuring the range
        let empty = String::new();
        let stdin_input = vars.get("start").unwrap_or(&empty);
        let mut gen = if stdin_input == "" {
            StdinCharGenerator::new(stdinlen, 0x20, 0x7e)
        } else {
            StdinCharGenerator::new_start(stdinlen, 0x20, 0x7e, stdin_input.as_bytes())
        };
        brute(path, 1, &mut gen, solver, terminal, vars.clone());

        let _ = file.write_fmt(format_args!("stdin: {}", gen));
    }
}
