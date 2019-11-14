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

use crate::b7tui::Ui;
use crate::brute::{brute, InstCounter};
use crate::errors::*;
use crate::generators::*;
use std::collections::HashMap;
use std::time::Duration;

use derive_setters::Setters;

/// Is B7 compiled for x86?
pub const IS_X86: bool = cfg!(target_arch = "x86") || cfg!(target_arch = "x86_64");

/// Simplified structure to consolidate all neccessary structs to run
///
/// # Example:
///
/// ```rust
/// // Solve wyvern with B7
/// B7Opts::new("wyvern")
///    .stdinstate(true)
///    .solver(Box::new(perf::PerfSolver))
///    .terminal(Box::new(Env::new()))
///    .vars(vars)
///    .timeout(Duration::from_secs(5))
///    .run();
/// ```
#[derive(Setters)]
pub struct B7Opts {
    /// Path to binary to solve
    #[setters(skip)]
    path: String,

    /// Initial input (default is `Input::new()`)
    init_input: Input,

    /// Whether to drop the ptrace connection after the process starts (default
    /// is `false`)
    drop_ptrace: bool,

    /// Whether to brute force argv (default is `false`)
    argstate: bool,

    /// Whether to brute force stdin (default is `false`)
    stdinstate: bool,

    /// The instruction counting engine to use (default is `perf::PerfSolver`)
    solver: Box<dyn InstCounter>,

    /// The UI to use (default is `b7tui::Env::new()`)
    terminal: Box<dyn Ui>,

    /// Timeout for each execution
    timeout: Duration,

    /// Misc variables
    vars: HashMap<String, String>,
}

impl B7Opts {
    pub fn new<T: AsRef<str>>(path: T) -> B7Opts {
        process::block_signal();
        B7Opts {
            path: path.as_ref().to_string(),
            init_input: Input::new(),
            drop_ptrace: false,
            argstate: false,
            stdinstate: false,
            solver: Box::new(perf::PerfSolver),
            terminal: Box::new(b7tui::Env::new()),
            vars: HashMap::new(),
            timeout: Duration::from_secs(1),
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
                &mut *self.terminal,
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
                &mut *self.terminal,
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
                &mut *self.terminal,
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
fn default_arg_brute(
    path: &str,
    init_input: &Input,
    solver: &dyn InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut dyn b7tui::Ui,
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
fn default_stdin_brute(
    path: &str,
    init_input: &Input,
    solver: &dyn InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut dyn b7tui::Ui,
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
fn default_mem_brute(
    path: &str,
    init_input: &Input,
    solver: &dyn InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut dyn b7tui::Ui,
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
