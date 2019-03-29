// use std::cmp::Ord;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::marker::Send;
use std::sync::mpsc::channel;
use std::sync::{Arc, Barrier};
use std::time::Duration;
use threadpool::ThreadPool;

use crate::b7tui;
use crate::errors::*;
use crate::generators::{Generate, Input};
use crate::statistics;
use crate::process::{WAITER, ProcessWaiter};

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
    get_inst_count: fn(&str, &Input, &HashMap<String, String>) -> Result<i64, SolverError>,
    terminal: &mut B,
    timeout: Duration,
    vars: HashMap<String, String>,
) -> Result<(), SolverError> {
    let n_workers = num_cpus::get();

    let pool = ThreadPool::new(n_workers);

    let barrier = Arc::new(Barrier::new(n_workers + 1));
    for i in 0..n_workers {
        let barrier = barrier.clone();
        pool.execute(move || {
            barrier.wait();
            WAITER.init_for_thread();
        });
    }

    barrier.wait();

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



        for inp_pair in data {
            num_jobs += 1;
            let tx = tx.clone();
            let test = String::from(path);
            // give it to a thread to handle
            let vars = vars.clone();

            pool.execute(move || {
                let inp = inp_pair.1;
                let mut inst_count = get_inst_count(&test, &inp, &vars);
                trace!("inst_count: {:?}", inst_count);
                for _ in 1..repeat {
                    inst_count = get_inst_count(&test, &inp, &vars);
                    trace!("inst_count: {:?}", inst_count);
                }
                let _ = tx.send((inp_pair.0, inst_count));
            });
        }
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
