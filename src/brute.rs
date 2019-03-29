// use std::cmp::Ord;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::marker::Send;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;
use scoped_pool::Pool;

use crate::b7tui;
use crate::errors::*;
use crate::generators::{Generate, Input};
use crate::statistics;

#[derive(Clone, Debug)]
pub struct InstCountData {
    pub path: String,
    pub inp: Input,
    pub vars: HashMap<String, String>,
    pub timeout: Duration
}

pub trait InstCounter: Send + Sync + 'static {
    fn get_inst_count(&self, data: &InstCountData) -> Result<i64, SolverError>;
}

// can take out Debug trait later
// Combines the generators with the instruction counters to deduce the next step
pub fn brute<
    G: Generate<I> + Display,
    I: 'static + std::fmt::Display + Clone + Debug + Send + Ord,
    B: b7tui::Ui,
>(
    path: &str,
    repeat: u32,
    gen: &mut G,
    counter: &InstCounter,
    terminal: &mut B,
    timeout: Duration,
    vars: HashMap<String, String>,
) -> Result<(), SolverError> {
    let n_workers = num_cpus::get();

    let pool = Pool::new(n_workers);

    // Loop until generator says we are done
    loop {
        // Number of threads to spawn

        let mut num_jobs: i64 = 0;
        let mut results: Vec<(I, i64)> = Vec::new();

        //let pool = ThreadPool
        let (tx, rx) = channel();


        let mut data = Vec::new();

        // run each case of the generators
        for inp_pair in gen.by_ref() {
            data.push(inp_pair);
        }

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
                    let inp = inp_pair.1;
                    let data = InstCountData {
                        path: test,
                        inp: inp,
                        vars: vars,
                        timeout
                    };
                    let mut inst_count = counter.get_inst_count(&data);
                    trace!("inst_count: {:?}", inst_count);
                    for _ in 1..repeat {
                        inst_count = counter.get_inst_count(&data);
                        trace!("inst_count: {:?}", inst_count);
                    }
                    let _ = tx.send((inp_pair.0, inst_count));
                });
            }
        });
        // Track the minimum for stats later
        let mut min: u64 = std::i64::MAX as u64;
        // Get results from the threads

        for _ in 0..num_jobs {
            let tmp = rx.recv().unwrap();
            match tmp.1 {
                Ok(x) => {
                    if (x as u64) < min {
                        min = x as u64;
                    }
                    results.push((tmp.0, x));
                }
                Err(x) => {
                    warn!("{:?} \n returned: {:?}", tmp.0, x);
                    continue;
                }

            }
        }
        results.sort();
        //println!("Got results: {:?}", results);

        terminal.update(&results, min);

        terminal.wait();

        // inform generator of the result
        if results.is_empty() {
            warn!("Results empty {:?}", results);
        }
        let good_idx = statistics::find_outlier(results.as_slice());
        if !gen.update(&good_idx.0) {
            break Ok(());
        }
    }
}
