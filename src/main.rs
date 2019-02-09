#[macro_use]
extern crate log;

use b7::*;

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

    let results = match &*terminal {
        "tui" => B7Opts::new(
            path.to_string(),
            argstate,
            stdinstate,
            solver,
            &mut b7tui::Tui::new(Some(String::from(path))),
            vars,
        )
        .run(),
        "env" => B7Opts::new(
            path.to_string(),
            argstate,
            stdinstate,
            solver,
            &mut b7tui::Env::new(),
            vars,
        )
        .run(),
        _ => panic!("unknown tui {}", terminal),
    };

    if let Some(s) = results.arg_brute {
        info!("Writing argv to cache");
        write!(file, "argv: {}", s).expect("Failed to write argv to cache!");
    };

    if let Some(s) = results.stdin_brute {
        info!("Writing stdin to cache");
        write!(file, "stdin: {}", s).expect("Failed to write stdin to cache!");
    };
}
