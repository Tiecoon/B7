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
    init_input: Input,
    argstate: bool,
    stdinstate: bool,
    solver: Box<dyn InstCounter>,
    terminal: &'a mut B,
    timeout: Duration,
    vars: HashMap<String, String>,
}

// TODO make into generators
/// human readable result
pub struct B7Results {
    pub arg_brute: String,
    pub stdin_brute: String,
    pub mem_brute: Vec<MemInput>,
}

impl<'a, B: b7tui::Ui> B7Opts<'a, B> {
    pub fn new(
        path: String,
        init_input: Input,
        // TODO make states into an enum
        argstate: bool,
        stdinstate: bool,
        solver: Box<dyn InstCounter>,
        terminal: &'a mut B,
        vars: HashMap<String, String>,
        timeout: Duration,
    ) -> B7Opts<'a, B> {
        process::block_signal();
        B7Opts {
            path,
            init_input,
            argstate,
            stdinstate,
            solver,
            terminal,
            vars,
            timeout,
        }
    }

    /// run b7 under given state and args
    pub fn run(&mut self) -> Result<B7Results, SolverError> {
        let mut arg_brute = String::new();
        let mut stdin_brute = String::new();
        let mut mem_brute = Vec::new();

        if self.argstate {
            arg_brute = default_arg_brute(
                &self.path,
                &self.init_input,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
            )?;
        }

        if self.stdinstate {
            stdin_brute = default_stdin_brute(
                &self.path,
                &self.init_input,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
            )?;
        }

        if !self.init_input.mem.is_empty() {
            mem_brute = default_mem_brute(
                &self.path,
                &self.init_input,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
            )?;
        }

        // let terminal decide if it should wait for user
        self.terminal.done();

        Ok(B7Results {
            arg_brute,
            stdin_brute,
            mem_brute,
        })
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
    init_input: &Input,
    solver: &dyn InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut B,
) -> Result<String, SolverError> {
    // Solve for argc
    let mut argcgen = ArgcGenerator::new(0, 5);
    brute(
        path,
        1,
        &mut argcgen,
        solver,
        init_input.clone(),
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
            init_input.clone(),
            terminal,
            timeout,
            vars.clone(),
        )?;
        let argvlens = argvlengen.get_lengths();

        // solve argv values
        let mut argvgen = ArgvGenerator::new(argc, argvlens, 0x20, 0x7e);
        brute(
            path,
            5,
            &mut argvgen,
            solver,
            init_input.clone(),
            terminal,
            timeout,
            vars.clone(),
        )?;

        return Ok(argvgen.to_string());
    }
    Ok(String::new()) //TODO should be an error
}

/// solves "default" stdin case
///
/// solves input ranges of
/// * `stdinlen` - 0-51
/// * `stdinchars` - 0x20-0x7e
fn default_stdin_brute<B: b7tui::Ui>(
    path: &str,
    init_input: &Input,
    solver: &dyn InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut B,
) -> Result<String, SolverError> {
    // solve stdin len
    let mut res = Input::new();
    let mut lgen = StdinLenGenerator::new(0, 51);
    brute(
        path,
        1,
        &mut lgen,
        solver,
        init_input.clone(),
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
        brute(
            path,
            1,
            &mut gen,
            solver,
            init_input.clone(),
            terminal,
            timeout,
            vars.clone(),
        )?;

        return Ok(gen.to_string());
    }
    Ok(String::new()) //TODO should be an error
}

/// Brute force memory regions and collect results
fn default_mem_brute<B: b7tui::Ui>(
    path: &str,
    init_input: &Input,
    solver: &dyn InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut B,
) -> Result<Vec<MemInput>, SolverError> {
    init_input
        .mem
        .iter()
        .map(|init_mem| {
            let mut gen = MemGenerator::new(init_mem.clone());

            brute(
                path,
                1,
                &mut gen,
                solver,
                init_input.clone(),
                terminal,
                timeout,
                vars.clone(),
            )?;

            Ok(gen.get_mem_input())
        })
        .collect::<Result<Vec<MemInput>, SolverError>>()
}
