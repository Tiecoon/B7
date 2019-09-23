#[macro_use]
extern crate log;

use b7::brute::InstCounter;
use b7::errors::*;
use b7::*;

use clap::{App, Arg};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::prelude::*;
use std::process::exit;
use std::time::Duration;

/// parses program arguements
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
            Arg::with_name("args")
                .help("Initial arguments to binary")
                .multiple(true),
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
        .arg(
            Arg::with_name("timeout")
                .long("timeout")
                .help("per-thread timeout to use when waiting for results, in seconds")
                .takes_value(true),
        )
        .get_matches()
}

/// output the help menu based on input
fn print_usage(matches: &clap::ArgMatches) -> ! {
    println!("{}", matches.usage());
    exit(-1);
}

fn main() -> Result<(), SolverError> {
    // handle command line arguements
    let matches = handle_cli_args();

    let path = match matches.value_of("binary") {
        Some(a) => a,
        None => print_usage(&matches),
    };

    let args = match matches.values_of_os("args") {
        Some(args) => args.map(OsStr::to_os_string).collect(),
        None => Vec::new(),
    };

    let argstate = matches.occurrences_of("argstate") < 1;
    let stdinstate = matches.occurrences_of("stdinstate") < 1;

    let solvername = matches.value_of("solver").unwrap_or("perf");
    let solver = match solvername {
        "perf" => Box::new(perf::PerfSolver) as Box<dyn InstCounter>,
        "dynamorio" => Box::new(dynamorio::DynamorioSolver) as Box<dyn InstCounter>,
        _ => panic!("unknown solver"),
    };
    let timeout = Duration::new(
        matches
            .value_of("timeout")
            .unwrap_or("5")
            .parse()
            .expect("Failed to parse duration!"),
        0,
    );

    let stdin_input = matches.value_of("start").unwrap_or("");
    let mut vars = HashMap::new();
    let dynpath = matches.value_of("dynpath").unwrap_or("");
    vars.insert(String::from("dynpath"), String::from(dynpath));
    vars.insert(String::from("stdininput"), String::from(stdin_input));

    let terminal = String::from(matches.value_of("ui").unwrap_or("tui")).to_lowercase();

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(format!("{}.cache", path))?;

    let results = match &*terminal {
        "tui" => B7Opts::new(
            path.to_string(),
            args,
            argstate,
            stdinstate,
            solver,
            &mut b7tui::Tui::new(Some(String::from(path))),
            vars,
            timeout,
        )
        .run(),
        "env" => B7Opts::new(
            path.to_string(),
            args,
            argstate,
            stdinstate,
            solver,
            &mut b7tui::Env::new(),
            vars,
            timeout,
        )
        .run(),
        _ => panic!("unknown tui {}", terminal),
    }?;

    if !results.arg_brute.is_empty() {
        info!("Writing argv to cache");
        write!(file, "argv: {}", results.arg_brute)?;
    };

    if !results.stdin_brute.is_empty() {
        info!("Writing stdin to cache");
        write!(file, "stdin: {}", results.stdin_brute)?;
    };
    Ok(())
}
