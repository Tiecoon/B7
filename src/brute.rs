// use std::cmp::Ord;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::marker::Send;
use std::time::Duration;
use std::sync::mpsc::channel;
use threadpool::ThreadPool;

use crate::b7tui;
use crate::generators::{Generate, Input};
use crate::statistics;

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
    get_inst_count: fn(&str, &Input, &HashMap<String, String>) -> i64,
    terminal: &mut B,
    timeout: Duration,
    vars: HashMap<String, String>,
) {
    // Loop until generator says we are done
    loop {
        // Number of threads to spawn
        let n_workers = num_cpus::get();
        let mut num_jobs: i64 = 0;
        let mut results: Vec<(I, i64)> = Vec::new();

        let pool = ThreadPool::new(n_workers);
        let (tx, rx) = channel();

        // run each case of the generators
        for inp_pair in gen.by_ref() {
            num_jobs += 1;
            let tx = tx.clone();
            let test = String::from(path);
            // give it to a thread to handle
            let vars = vars.clone();
            pool.execute(move || {
                let inp = inp_pair.1;
                let mut avg: f64 = 0.0;
                let mut count: f64 = 0.0;
                for _ in 0..repeat {
                    let inst_count = get_inst_count(&test, &inp, &vars);
                    avg += inst_count as f64;
                    count += 1.0;
                    trace!("inst_count: {:?}", inst_count);
                }
                avg /= count;
                let _ = tx.send((inp_pair.0, avg as i64));
            });
        }
        // Track the minimum for stats later
        let mut min: u64 = std::i64::MAX as u64;
        // Get results from the threads
        for _ in 0..num_jobs {
            match rx.recv_timeout(timeout) {
                Ok(tmp) => {
                    if (tmp.1 as u64) < min {
                        min = tmp.1 as u64;
                    }
                    results.push(tmp);
                }
                Err(_) => {
                    error!("timeout!") // TODO: print inpit
                }
            }
        }
        results.sort();

        terminal.update(&results, min);

        terminal.wait();

        // inform generator of the result
        let good_idx = statistics::find_outlier(results.as_slice());
        if !gen.update(&good_idx.0) {
            break;
        }
    }
}
