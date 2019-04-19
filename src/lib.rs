#[macro_use]
extern crate log;

pub mod b7tui;
pub mod binary;
pub mod bindings;
pub mod brute;
pub mod dynamorio;
pub mod errors;
pub mod generators;
pub mod perf;
pub mod process;
pub mod statistics;

use crate::brute::{brute, InstCounter};
use crate::errors::*;
use crate::generators::*;
use std::collections::HashMap;
use std::time::Duration;

/// simpified structure to consolate all neccessary structs to run
pub struct B7Opts<'a, B: b7tui::Ui> {
    path: String,
    argstate: bool,
    stdinstate: bool,
    solver: Box<InstCounter>,
    terminal: &'a mut B,
    timeout: Duration,
    vars: HashMap<String, String>,
}

// TODO make into generators
/// human readable result
pub struct B7Results {
    pub arg_brute: String,
    pub stdin_brute: String,
}

impl<'a, B: b7tui::Ui> B7Opts<'a, B> {
    pub fn new(
        path: String,
        // TODO make states into an enum
        argstate: bool,
        stdinstate: bool,
        solver: Box<InstCounter>,
        terminal: &'a mut B,
        vars: HashMap<String, String>,
        timeout: Duration,
    ) -> B7Opts<'a, B> {
        process::block_signal();
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

    /// run b7 under given state and args
    pub fn run(&mut self) -> Result<Input, SolverError> {
        let mut res = Input::new();
        if self.argstate {
            res = res.combine(default_arg_brute(
                &self.path,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
            )?);
        }

        if self.stdinstate {
            res = res.combine(default_stdin_brute(
                &self.path,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
            )?);
        }

        // let terminal decide if it should wait for user
        self.terminal.done();

        Ok(res)
    }
}

/// solves "default" arguement case
///
/// solves input ranges of
/// * `argc` - 0-5
/// * `argvlength` - 0-20
/// * `argvchars` - 0x20-0x7e (standard ascii char range)
fn default_arg_brute<B: b7tui::Ui>(
    path: &str,
    solver: &InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut B,
) -> Result<Input, SolverError> {
    // Solve for argc
    let mut argcgen = ArgcGenerator::new(0, 5);
    brute(
        path,
        1,
        &mut argcgen,
        solver,
        Input::new(),
        terminal,
        timeout,
        vars.clone(),
    )?;
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
            Input::new(),
            terminal,
            timeout,
            vars.clone(),
        )?;
        let argvlens = argvlengen.get_lengths();

        // solve argv values
        let mut argvgen = ArgvGenerator::new(argc, argvlens, 0x20, 0x7e);
        let res = brute(
            path,
            5,
            &mut argvgen,
            solver,
            Input::new(),
            terminal,
            timeout,
            vars.clone(),
        )?;

        return Ok(res);
    }
    Err(SolverError::new(Runner::RunnerError, "arg brute failed")) //TODO should be an error
}

/// solves "default" stdin case
///
/// solves input ranges of
/// * `stdinlen` - 0-51
/// * `stdinchars` - 0x20-0x7e
fn default_stdin_brute<B: b7tui::Ui>(
    path: &str,
    solver: &InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut B,
) -> Result<Input, SolverError> {
    // solve stdin len
    let mut res = Input::new();
    let mut lgen = StdinLenGenerator::new(0, 51);
    brute(
        path,
        1,
        &mut lgen,
        solver,
        Input::new(),
        terminal,
        timeout,
        vars.clone(),
    )?;
    res.stdinlen = lgen.get_length();
    // solve strin if there is stuff to solve
    if res.stdinlen > 0 {
        // TODO: We should have a good way of configuring the range
        let empty = String::new();
        let stdin_input = vars.get("start").unwrap_or(&empty);
        let mut gen = if stdin_input == "" {
            StdinCharGenerator::new(res, 0x20, 0x7e)
        } else {
            StdinCharGenerator::new_start(res, 0x20, 0x7e, stdin_input.as_bytes())
        };
        let stdin = brute(
            path,
            1,
            &mut gen,
            solver,
            Input::new(),
            terminal,
            timeout,
            vars.clone(),
        )?;

        return Ok(stdin);
    }
    Err(SolverError::new(
        Runner::RunnerError,
        "stdin generator failed",
    ))
}
