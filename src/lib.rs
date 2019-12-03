//! Features
//! * dynamorio
//!     * will compile dynamorio in build_* and the code to allow its use

#[macro_use]
extern crate log;
extern crate env_logger;

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
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use derive_setters::Setters;

/// Is B7 compiled for x86?
pub const IS_X86: bool = cfg!(target_arch = "x86") || cfg!(target_arch = "x86_64");

/// Options to pass to B7
///
/// Example:
///
/// ```rust
/// # use b7::B7Opts;
/// # use std::time::Duration;
/// let res = B7Opts::new("tests/bins/wyvern")
///     .solve_stdin(true)
///     .timeout(Duration::from_secs(5))
///     .run()
///     .unwrap();
/// ```
#[derive(Setters)]
pub struct B7Opts {
    /// Path to binary
    #[setters(skip)]
    path: PathBuf,

    /// Initial input to pass to binary (default: `Input::new()`)
    init_input: Input,

    /// Whether to drop ptrace connection (default: `false`)
    drop_ptrace: bool,

    /// Whether to brute force argv (default: `false`)
    solve_argv: bool,

    /// Whether to brute force stdin (default: `false`)
    solve_stdin: bool,

    /// Which solver to use (default: `Box::new(b7::perf::PerfSolver)`)
    solver: Box<dyn InstCounter>,

    /// Which UI to use (default: `Box::new(b7::b7tui::Env::new()`)
    ui: Box<dyn Ui>,

    /// Timeout for each run (default: `Duration::from_secs(1)`)
    timeout: Duration,

    /// Misc variables (default: `HashMap::new()`)
    vars: HashMap<String, String>,
}

impl B7Opts {
    pub fn new<T: AsRef<Path>>(path: T) -> B7Opts {
        process::block_signal();
        B7Opts {
            path: path.as_ref().to_path_buf(),
            init_input: Input::new(),
            drop_ptrace: false,
            solve_argv: false,
            solve_stdin: false,
            solver: Box::new(perf::PerfSolver),
            ui: Box::new(b7tui::Env::new()),
            vars: HashMap::new(),
            timeout: Duration::from_secs(1),
        }
    }

    /// run b7 under given state and args
    pub fn run(&mut self) -> Result<Input, SolverError> {
        debug!("Executing run: {:?}", self.init_input);
        let mut solved = self.init_input.clone();

        if self.solve_argv {
            solved = default_arg_brute(
                &self.path,
                &solved,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                &mut *self.ui,
                self.drop_ptrace,
            )?;
        }

        if self.solve_stdin {
            solved = default_stdin_brute(
                &self.path,
                &solved,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                &mut *self.ui,
                self.drop_ptrace,
            )?;
        }

        if self.init_input.mem.is_some() {
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
                &mut *self.ui,
            )?;
        }

        // let UI decide if it should wait for user
        self.ui.done();

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
    path: &Path,
    init_input: &Input,
    solver: &dyn InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut dyn b7tui::Ui,
    drop_ptrace: bool,
) -> Result<Input, SolverError> {
    terminal.set_timeout(timeout);
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
        vars.clone(),
        drop_ptrace,
    )?;

    // check if there is something to be solved
    if let Some(argc) = init_input.argc {
        if argc > 0 {
            // solve argv length
            let mut argvlengen = ArgvLenGenerator::new(argc, 0, 20);
            solved = brute(
                path,
                5,
                &mut argvlengen,
                solver,
                solved,
                terminal,
                vars.clone(),
                drop_ptrace,
            )?;

            // solve argv values
            if let Some(argvlens) = init_input.argvlens.clone() {
                let mut argvgen = ArgvGenerator::new(argc, argvlens.as_slice(), 0x20, 0x7e);
                solved = brute(
                    path,
                    5,
                    &mut argvgen,
                    solver,
                    solved,
                    terminal,
                    vars.clone(),
                    drop_ptrace,
                )?;
            }

            return Ok(solved);
        }
    }
    Ok(solved)
}

/// solves "default" stdin case
///
/// solves input ranges of
/// * `stdinlen` - 0-51
/// * `stdinchars` - 0x20-0x7e
fn default_stdin_brute(
    path: &Path,
    init_input: &Input,
    solver: &dyn InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut dyn b7tui::Ui,
    drop_ptrace: bool,
) -> Result<Input, SolverError> {
    terminal.set_timeout(timeout);
    // solve stdin len if unspecified
    let mut solved = init_input.clone();
    if solved.stdinlen.is_none() {
        solved = brute(
            path,
            1,
            &mut StdinLenGenerator::new(0, 51),
            solver,
            solved,
            terminal,
            vars.clone(),
            drop_ptrace,
        )?;
    }
    // solve stdin if there is stuff to solve
    if solved.stdinlen.is_some() {
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
            vars.clone(),
            drop_ptrace,
        )?);
    }
    Ok(solved)
}

/// Brute force memory regions and collect results
fn default_mem_brute(
    path: &Path,
    init_input: &Input,
    solver: &dyn InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut dyn b7tui::Ui,
) -> Result<Input, SolverError> {
    terminal.set_timeout(timeout);
    let original = init_input.clone();
    let mem = match original.mem {
        Some(i) => i,
        None => return Err(SolverError::new(Runner::NoneError, "No memory to run")),
    };

    let mut solved = init_input.clone();
    for input in mem {
        let mut gen = MemGenerator::new(input.clone());

        solved = brute(
            path,
            1,
            &mut gen,
            solver,
            solved.clone(),
            terminal,
            vars.clone(),
            false,
        )?;
    }
    Ok(solved)
}
