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
use std::collections::HashMap;
use std::time::Duration;

pub type Solver = fn(&str, &Input, &HashMap<String, String>) -> i64;

pub struct B7Opts<'a, B: b7tui::Ui> {
    path: String,
    argstate: bool,
    stdinstate: bool,
    solver: Solver,
    terminal: &'a mut B,
    timeout: Duration,
    vars: HashMap<String, String>,
}

pub struct B7Results {
    pub arg_brute: Option<String>,
    pub stdin_brute: Option<String>,
}

impl<'a, B: b7tui::Ui> B7Opts<'a, B> {
    pub fn new(
        path: String,
        argstate: bool,
        stdinstate: bool,
        solver: Solver,
        terminal: &'a mut B,
        vars: HashMap<String, String>,
        timeout: Duration,
    ) -> B7Opts<'a, B> {
        B7Opts {
            path,
            argstate,
            stdinstate,
            solver,
            terminal,
            vars,
            timeout,
        }
    }

    pub fn run(&mut self) -> B7Results {
        let mut arg_brute = None;
        let mut stdin_brute = None;
        if self.argstate {
            arg_brute = default_arg_brute(
                &self.path,
                self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
            );
        }

        if self.stdinstate {
            stdin_brute = default_stdin_brute(
                &self.path,
                self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
            );
        }

        // let terminal decide if it should wait for user
        self.terminal.done();

        B7Results {
            arg_brute,
            stdin_brute,
        }
    }
}

// solves "default" arguement case
fn default_arg_brute<B: b7tui::Ui>(
    path: &str,
    solver: fn(&str, &Input, &HashMap<String, String>) -> i64,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut B,
) -> Option<String> {
    // Solve for argc
    let mut argcgen = ArgcGenerator::new(0, 5);
    brute(
        path,
        1,
        &mut argcgen,
        solver,
        terminal,
        timeout,
        vars.clone(),
    );
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
            terminal,
            timeout,
            vars.clone(),
        );
        let argvlens = argvlengen.get_lengths();

        // solve argv values
        let mut argvgen = ArgvGenerator::new(argc, argvlens, 0x20, 0x7e);
        brute(
            path,
            5,
            &mut argvgen,
            solver,
            terminal,
            timeout,
            vars.clone(),
        );

        return Some(argvgen.to_string());
    }
    None
}

// solves "default" stdin case
fn default_stdin_brute<B: b7tui::Ui>(
    path: &str,
    solver: fn(&str, &Input, &HashMap<String, String>) -> i64,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut B,
) -> Option<String> {
    // solve stdin len
    let mut lgen = StdinLenGenerator::new(0, 51);
    brute(path, 1, &mut lgen, solver, terminal, timeout, vars.clone());
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
        brute(path, 1, &mut gen, solver, terminal, timeout, vars.clone());

        return Some(gen.to_string());
    }
    None
}
