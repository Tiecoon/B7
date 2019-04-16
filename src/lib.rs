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

use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;



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

/// Holds the current state of all generators
/// This can be serialized and deserialized to/from disk,
/// to support resuming B7 from previous state
#[derive(Serialize, Deserialize)]
pub struct B7State {
    pub arg_state: Option<ArgState>,
    pub stdin_state: Option<StdinState>
}

impl B7State {
    fn new(use_args: bool, use_stdin: bool) -> B7State {
        let mut arg_state = None;
        let mut stdin_state = None;

        if use_args {
            arg_state = Some(ArgState::default());
        }
        if use_stdin {
            stdin_state = Some(StdinState::default());
        }

        B7State {
            arg_state,
            stdin_state
        }
    }

    fn run<B: b7tui::Ui>(
        &mut self,
        path: &str,
        solver: &InstCounter,
        vars: HashMap<String, String>,
        timeout: Duration,
        terminal: &mut B,
    ) -> Result<B7Results, SolverError> {
        let arg_brute = self.arg_state.as_mut().map(|s| s.run(path, solver, vars.clone(), timeout, terminal))
            .unwrap_or_else(|| Ok(String::new()))?;

        let stdin_brute = self.stdin_state.as_mut().map(|s| s.run(path, solver, vars.clone(), timeout, terminal))
            .unwrap_or_else(|| Ok(String::new()))?;

        Ok(B7Results {
            arg_brute,
            stdin_brute
        })

    }

}


trait GeneratorState: Serialize + DeserializeOwned {
    fn run<B: b7tui::Ui>(
        &mut self,
        path: &str,
        solver: &InstCounter,
        vars: HashMap<String, String>,
        timeout: Duration,
        terminal: &mut B,
    ) -> Result<String, SolverError>;
}

#[derive(Serialize, Deserialize)]
pub enum ArgState {
    Argc(ArgcGenerator),
    ArgvLen(ArgvLenGenerator),
    Argv(ArgvGenerator),
    Done(String)
    /*argcgen: ArgcGenerator,
    argvlengen: Option<ArgvLenGenerator>,
    argvgen: Option<ArgvGenerator>*/
}


impl Default for ArgState {
    fn default() -> Self {
        ArgState::Argc(ArgcGenerator::new(0, 5))
    }
}

impl GeneratorState for ArgState {
    fn run<B: b7tui::Ui>(
        &mut self,
        path: &str,
        solver: &InstCounter,
        vars: HashMap<String, String>,
        timeout: Duration,
        terminal: &mut B,
    ) -> Result<String, SolverError> {
        loop {
            match self {
                &mut ArgState::Argc(ref mut gen) => {
                    brute(path, 1, gen, solver, terminal, timeout, vars.clone())?;
                    let argc = gen.get_length();
                    if argc == 0 {
                        //TODO should be an error
                        *self = ArgState::Done(String::new());
                        continue;
                    } else {
                        let mut argvlengen = ArgvLenGenerator::new(argc, 0, 20);
                        *self = ArgState::ArgvLen(argvlengen);
                    }
                },
                &mut ArgState::ArgvLen(ref mut gen) => {
                    brute(path, 5, gen, solver, terminal, timeout, vars.clone())?;
                    let argc = gen.get_argc();
                    let argvlens = gen.get_lengths();
                    *self = ArgState::Argv(ArgvGenerator::new(argc, argvlens, 0x20, 0x7e));
                },
                &mut ArgState::Argv(ref mut gen) => {
                    brute(path, 5, gen, solver, terminal, timeout, vars.clone())?;
                    *self = ArgState::Done(gen.to_string());
                }
                &mut ArgState::Done(ref s) => {
                    return Ok(s.clone());
                }
            }
        }
    }

}


#[derive(Serialize, Deserialize)]
pub enum StdinState {
    StdinLen(StdinLenGenerator),
    StdinGen(StdinCharGenerator),
    Done(String)
    //lgen: StdinLenGenerator,
    //gen: Option<StdinCharGenerator>
}

impl Default for StdinState {
    fn default() -> Self {
        StdinState::StdinLen(StdinLenGenerator::new(0, 51))
    }
}

impl GeneratorState for StdinState {
    fn run<B: b7tui::Ui>(
        &mut self,
        path: &str,
        solver: &InstCounter,
        vars: HashMap<String, String>,
        timeout: Duration,
        terminal: &mut B,
    ) -> Result<String, SolverError> {
        loop {
            match self {
                &mut StdinState::StdinLen(ref mut gen) => {
                    brute(path, 1, gen, solver, terminal, timeout, vars.clone())?;
                    let stdinlen = gen.get_length();

                    if stdinlen == 0 {
                        //TODO should be an error
                        *self = StdinState::Done(String::new());
                    } else {
                        let empty = String::new();
                        let stdin_input = vars.get("start").unwrap_or(&empty);
                        let mut gen = if stdin_input == "" {
                            StdinCharGenerator::new(stdinlen, 0x20, 0x7e)
                        } else {
                            StdinCharGenerator::new_start(stdinlen, 0x20, 0x7e, stdin_input.as_bytes())
                        };

                        *self = StdinState::StdinGen(gen);
                    }
                },
                &mut StdinState::StdinGen(ref mut gen) => {
                    brute(path, 1, gen, solver, terminal, timeout, vars.clone())?;
                    *self = StdinState::Done(gen.to_string());
                },
                &mut StdinState::Done(ref s) => {
                    return Ok(s.clone())
                }
            }
        }
    }

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
    pub fn run(&mut self) -> Result<B7Results, SolverError> {
        let mut arg_brute = String::new();
        let mut stdin_brute = String::new();

        let mut state = B7State::new(self.argstate, self.stdinstate);
        let res = state.run(&self.path, &*self.solver, self.vars.clone(), self.timeout, self.terminal)?;

        /*if self.argstate {
            arg_brute = default_arg_brute(
                &self.path,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
            )?;
        }

        if self.stdinstate {
            stdin_brute = default_stdin_brute(
                &self.path,
                &*self.solver,
                self.vars.clone(),
                self.timeout,
                self.terminal,
            )?;
        }*/

        // let terminal decide if it should wait for user
        self.terminal.done();

        Ok(res)

        /*Ok(B7Results {
            arg_brute,
            stdin_brute,
        })*/
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
) -> Result<String, SolverError> {
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
    solver: &InstCounter,
    vars: HashMap<String, String>,
    timeout: Duration,
    terminal: &mut B,
) -> Result<String, SolverError> {
    // solve stdin len
    let mut lgen = StdinLenGenerator::new(0, 51);
    brute(path, 1, &mut lgen, solver, terminal, timeout, vars.clone())?;
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
        brute(path, 1, &mut gen, solver, terminal, timeout, vars.clone())?;

        return Ok(gen.to_string());
    }
    Ok(String::new()) //TODO should be an error
}
