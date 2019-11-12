//! Features
//! * dynamorio
//!     * will compile dynamorio in build_* and the code to allow its use

#[macro_use]
extern crate log;

pub mod b7tui;
pub mod binary;
pub mod bindings;
pub mod brute;
#[cfg(feature = "dynamorio")]
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

/// Is B7 compiled for x86?
pub const IS_X86: bool = cfg!(target_arch = "x86") || cfg!(target_arch = "x86_64");

/// simpified structure to consolate all neccessary structs to run
pub struct B7Opts<'a, B: b7tui::Ui> {
    path: String,
    init_input: Input,
    drop_ptrace: bool,
    argstate: bool,
    stdinstate: bool,
    solver: Box<dyn InstCounter>,
    terminal: &'a mut B,
    timeout: Duration,
    vars: HashMap<String, String>,
}

impl<'a, B: b7tui::Ui> B7Opts<'a, B> {
    pub fn new(
        path: String,
        init_input: Input,
        // TODO make states into an enum
        drop_ptrace: bool,
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
            drop_ptrace,
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
        let mut solved = self.init_input.clone();

        if self.argstate {
            solved = default_arg_brute(
                &self.path,
                &solved,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
                self.drop_ptrace,
            )?;
        }

        if self.stdinstate {
            solved = default_stdin_brute(
                &self.path,
                &solved,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
                self.drop_ptrace,
            )?;
        }

        if !self.init_input.mem.is_empty() {
            if self.drop_ptrace {
                return Err(SolverError::new(
                    Runner::ArgError,
                    "ptrace dropping and mem input are mutually exclusive",
                ));
            }

            solved = default_mem_brute(
                &self.path,
                &solved,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
            )?;
        }

        // let terminal decide if it should wait for user
        self.terminal.done();

        Ok(solved)
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
    drop_ptrace: bool,
) -> Result<Input, SolverError> {
    let mut solved = init_input.clone();
    // Solve for argc
    let mut argcgen = ArgcGenerator::new(0, 5);
    solved = brute(
        path,
        1,
        &mut argcgen,
        solver,
        solved,
        terminal,
        timeout,
        vars.clone(),
        drop_ptrace,
    )?;

    // check if there is something to be solved
    if init_input.argc > 0 {
        // solve argv length
        let mut argvlengen = ArgvLenGenerator::new(init_input.argc, 0, 20);
        solved = brute(
            path,
            5,
            &mut argvlengen,
            solver,
            solved,
            terminal,
            timeout,
            vars.clone(),
            drop_ptrace,
        )?;

        // solve argv values
        let mut argvgen =
            ArgvGenerator::new(init_input.argc, init_input.argvlens.as_slice(), 0x20, 0x7e);
        let solved = brute(
            path,
            5,
            &mut argvgen,
            solver,
            solved,
            terminal,
            timeout,
            vars.clone(),
            drop_ptrace,
        )?;

        return Ok(solved);
    }
    Ok(solved)
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
    drop_ptrace: bool,
) -> Result<Input, SolverError> {
    // solve stdin len if unspecified
    let mut solved = init_input.clone();
    if solved.stdinlen == 0 {
        solved = brute(
            path,
            1,
            &mut StdinLenGenerator::new(0, 51),
            solver,
            solved,
            terminal,
            timeout,
            vars.clone(),
            drop_ptrace,
        )?;
    }
    // solve stdin if there is stuff to solve
    if solved.stdinlen > 0 {
        // TODO: We should have a good way of configuring the range
        let empty = String::new();
        let stdin_input = vars.get("start").unwrap_or(&empty);
        let mut gen = if stdin_input == "" {
            StdinCharGenerator::new(solved.clone(), 0x20, 0x7e)
        } else {
            StdinCharGenerator::new_start(solved.clone(), 0x20, 0x7e, stdin_input.as_bytes())
        };
        return Ok(brute(
            path,
            1,
            &mut gen,
            solver,
            solved.clone(),
            terminal,
            timeout,
            vars.clone(),
            drop_ptrace,
        )?);
    }
    Ok(solved)
}

/// Brute force memory regions and collect results
fn default_mem_brute<B: b7tui::Ui>(
    path: &str,
    init_input: &Input,
    solver: &dyn InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut B,
) -> Result<Input, SolverError> {
    let original = init_input.clone();
    let mut solved = init_input.clone();
    for input in original.mem {
        let mut gen = MemGenerator::new(input.clone());

        solved = brute(
            path,
            1,
            &mut gen,
            solver,
            solved.clone(),
            terminal,
            timeout,
            vars.clone(),
            false,
        )?;
    }
    Ok(solved)
}
