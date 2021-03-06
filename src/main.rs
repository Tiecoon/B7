extern crate env_logger;
extern crate log;

use b7::brute::InstCounter;
use b7::errors::*;
use b7::generators::Input;
use b7::generators::MemInput;
use b7::*;

use clap::{App, Arg};
use log::debug;
use std::collections::HashMap;
use std::os::unix::ffi::OsStrExt;
use std::process::exit;
use std::time::Duration;

use is_executable::IsExecutable;

/// Parse memory inputs from args
fn mem_inputs_from_args(matches: &clap::ArgMatches) -> SolverResult<Option<Vec<MemInput>>> {
    debug!("Executing mem_inputs_from_args:");
    match matches.values_of("mem-brute") {
        Some(x) => x.map(MemInput::parse_from_arg).collect(),
        None => Ok(None),
    }
}

/// parses program arguements
fn handle_cli_args<'a>() -> clap::ArgMatches<'a> {
    debug!("Executing handle_cli_args:");
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
            Arg::with_name("stdin-len")
                .long("stdin-len")
                .help("specify stdin length")
                .takes_value(true),
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
        .arg(
            Arg::with_name("mem-brute")
                .long("mem-brute")
                .help(
                    "Address, size, and initial input (optional) of memory \
                     buffer to brute force. For PIE binaries, the address is \
                     relative to the executable base. Otherwise, the address is \
                     absolute.\
                     \n    Example: `--mem-brute \
                     addr=404060,size=64,init=666c61677b0a`",
                )
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("drop-ptrace")
                .long("drop-ptrace")
                .conflicts_with("mem-brute")
                .help(
                    "detach from ptrace after the binary starts (use this if the \
                     binary is movfuscated, has frequently-triggered signal \
                     handlers, or uses ptrace anti-debugging)",
                ),
        )
        .get_matches()
}

/// output the help menu based on input
fn print_usage(matches: &clap::ArgMatches) -> ! {
    debug!("Executing print_usage:");
    println!("{}", matches.usage());
    exit(-1);
}

fn main() -> Result<(), SolverError> {
    debug!("Executing main:");
    // handle command line arguements
    let matches = handle_cli_args();

    let path = match matches.value_of("binary") {
        Some(a) => a,
        None => print_usage(&matches),
    };

    if !std::path::Path::new(path).is_executable() {
        panic!("File type provided is not executable.");
    }

    let args = match matches.values_of_os("args") {
        Some(args) => Some(args.map(|arg| arg.as_bytes().to_vec()).collect()),
        None => None,
    };

    let drop_ptrace = matches.is_present("drop-ptrace");
    let argstate = matches.occurrences_of("argstate") < 1;
    let stdinstate = matches.occurrences_of("stdinstate") < 1;
    let stdinlen = match matches.value_of("stdin-len") {
        Some(x) => Some(x.parse::<u32>().expect("invalid stdin length")),
        None => None,
    };

    let solvername = matches.value_of("solver").unwrap_or("perf");
    let solver = match solvername {
        "perf" => Box::new(perf::PerfSolver) as Box<dyn InstCounter>,
        #[cfg(feature = "dynamorio")]
        "dynamorio" => Box::new(dynamorio::DynamorioSolver) as Box<dyn InstCounter>,
        _ => panic!("unknown solver"),
    };
    let timeout = Duration::from_secs(
        matches
            .value_of("timeout")
            .unwrap_or("5")
            .parse()
            .expect("Failed to parse duration!"),
    );

    let stdin_input = matches.value_of("start").unwrap_or("");
    let mut vars = HashMap::new();
    let dynpath = matches.value_of("dynpath").unwrap_or("");
    vars.insert(String::from("dynpath"), String::from(dynpath));
    vars.insert(String::from("stdininput"), String::from(stdin_input));

    let ui = String::from(matches.value_of("ui").unwrap_or("tui")).to_lowercase();
    let ui = match &*ui {
        "tui" => Box::new(b7tui::Tui::new(Some(String::from(path)))) as Box<dyn b7tui::Ui>,
        "env" => Box::new(b7tui::Env::new()) as Box<dyn b7tui::Ui>,
        _ => panic!("unknown UI {}", ui),
    };

    let _results = B7Opts::new(path)
        .init_input(Input {
            stdinlen,
            argv: args,
            mem: mem_inputs_from_args(&matches)?,
            ..Default::default()
        })
        .drop_ptrace(drop_ptrace)
        .solve_argv(argstate)
        .solve_stdin(stdinstate)
        .solver(solver)
        .ui(ui)
        .vars(vars)
        .timeout(timeout)
        .run()?;

    Ok(())
}
