// use std::cmp::Ord;
use scoped_pool::Pool;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::{Debug, Display};
use std::marker::Send;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;

use crate::b7tui;
use crate::errors::*;
use crate::generators::{Generate, Input};
use crate::statistics;

#[derive(Clone, Debug)]
/// holds information that is universal to InstCounters
pub struct InstCountData {
    pub path: String,
    pub args: Vec<OsString>,
    pub inp: Input,
    pub vars: HashMap<String, String>,
    pub timeout: Duration,
}

pub trait InstCounter: Send + Sync + 'static {
    /// function that runs a program and returns a number representing progress in a binary
    /// runs passed on info in data
    fn get_inst_count(&self, data: &InstCountData) -> Result<i64, SolverError>;
}

// can take out Debug trait later
/// Combines the generators with the instruction counters to deduce the next input.
/// Responsible for spinning up threads and managing process input
///
/// # Arguements
///
/// * `path` - a string slice that holds the path to the binary
/// * `args` - a slice of OS strings with arguments that are always passed first
/// * `repeat` - an int to tell how many runs to average for each input
/// * `gen` - a generators::generator that has Display trait to use to generate additional input
/// * `counter` - the inst_counter function to run the binary under
/// * `Solved` - other constraints to pass to the binary
/// * `terminal` - a b7tui::Ui to present data to, so it can display it
/// * `timeout` - a duration in seconds to timeout program after
/// * `vars` - additional variables that the counter function might need
///
/// # Example
///
/// ```no_run
/// # use crate::b7::errors::*;
/// # use crate::b7::generators;
/// # use crate::b7::perf;
/// # use crate::b7::generators::Input;
/// # use crate::b7::b7tui;
/// # use b7::brute::brute;
/// # use b7::brute::InstCounter;
/// use std::collections::HashMap;
/// use std::time::Duration;
/// use std::io;
/// fn main() -> Result<(), SolverError> {
///
///    let mut task = generators::ArgcGenerator::new(0,9);
///
///    brute(
///        "./tests/wyvern",
///        &[],
///        1,
///        &mut task,
///        &perf::PerfSolver,
///        Input::new(),
///        &mut b7tui::Env,
///        Duration::new(5,0),
///        HashMap::new(),
///    )?;
///
///    // prints the number of argc it found
///    println!("argc is: {}",task);
///
///    Ok(())
/// }
/// ```
pub fn brute<
    G: Generate<I> + Display,
    I: 'static + std::fmt::Display + Clone + Debug + Send + Ord,
    B: b7tui::Ui,
>(
    path: &str,
    args: &[OsString],
    repeat: u32,
    gen: &mut G,
    counter: &dyn InstCounter,
    solved: Input,
    terminal: &mut B,
    timeout: Duration,
    vars: HashMap<String, String>,
) -> Result<Input, SolverError> {
    let n_workers = num_cpus::get();

    let pool = Pool::new(n_workers);

    // Loop until generator says we are done
    loop {
        // Number of threads to spawn

        let mut num_jobs: i64 = 0;

        let (tx, rx) = channel();

        let mut data = Vec::new();

        // run each case of the generators
        for inp_pair in gen.by_ref() {
            data.push((inp_pair.0, solved.clone().combine(inp_pair.1)));
        }
        let mut results: Box<Vec<(i64, (I, Input))>> = Box::new(Vec::with_capacity(data.len()));
        let counter = Arc::new(counter);

        pool.scoped(|scope| {
            for inp_pair in data {
                num_jobs += 1;
                let tx = tx.clone();
                let test = String::from(path);
                // give it to a thread to handle
                let vars = vars.clone();
                let counter = counter.clone();

                scope.execute(move || {
                    let inp = (inp_pair.1).clone();
                    let data = InstCountData {
                        path: test,
                        args: args.to_vec(),
                        inp,
                        vars,
                        timeout,
                    };
                    let mut inst_count = counter.get_inst_count(&data);
                    trace!("inst_count: {:?}", inst_count);
                    for _ in 1..repeat {
                        inst_count = counter.get_inst_count(&data);
                        trace!("inst_count: {:?}", inst_count);
                    }
                    let _ = tx.send((inst_count, (inp_pair)));
                });
            }
        });
        // Track the minimum for stats later
        let mut min: u64 = std::i64::MAX as u64;
        // Get results from the threads

        for _ in 0..num_jobs {
            let tmp = rx.recv().unwrap();
            match tmp.0 {
                Ok(x) => {
                    if (x as u64) < min {
                        min = x as u64;
                    }
                    results.push((x, (tmp.1)));
                }
                Err(x) => {
                    warn!("{:?} \n returned: {:?}", (tmp.1).0, x);
                    continue;
                }
            }
        }
        results.shrink_to_fit();
        terminal.update(results.clone(), min);

        terminal.wait();

        // inform generator of the result
        if results.is_empty() {
            warn!("Results empty {:?}", results);
            return Err(SolverError::new(Runner::Unknown, "No valid results found"));
        }
        let good_idx = statistics::find_outlier(results.as_slice());
        if !gen.update(&(good_idx.1).0) {
            break Ok((good_idx.1).1.clone());
        }
    }
}
