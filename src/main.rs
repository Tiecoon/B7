extern crate clap;
extern crate libc;
extern crate nix;
extern crate spawn_ptrace;
extern crate threadpool;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate termion;
extern crate tui;

use std::io;

use clap::{App, Arg};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Style};
use tui::widgets::{BarChart, Block, Borders, Widget};
use tui::Terminal;
extern crate tui_logger;

use log::LevelFilter;
use tui_logger::*;

use std::sync::mpsc::channel;
use threadpool::ThreadPool;

pub mod binary;
pub mod bindings;

pub mod process;
use process::Process;

pub mod generators;
use generators::*;

pub mod b7777;

use std::{thread, time};

fn get_inst_count_perf(path: &str, inp: &Input) -> i64 {
    // TODO: error checking...
    let mut proc = Process::new(path);
    for arg in inp.argv.iter() {
        proc.arg(OsStr::from_bytes(arg));
    }
    proc.start().unwrap();
    proc.write_stdin(&inp.stdin).unwrap();
    proc.close_stdin().unwrap();
    proc.init_perf().unwrap();
    proc.finish().unwrap();
    let ret = match proc.get_inst_count() {
        Ok(x) => x,
        Err(_) => -1,
    };
    proc.close_perf();
    ret
}

// Find the most distant point from the average.

// Returns (index, value) of this point. (TODO: fix this)
fn find_outlier<I: std::fmt::Debug>(counts: &[(I, i64)]) -> &(I, i64) {
    // Calculate the average
    let mut avg: i64 = 0;
    for (_, count) in counts.iter() {
        avg += count;
    }
    if !counts.is_empty() {
        avg /= counts.len() as i64;
    } else {
        // Handle division by zero
        warn!("WWWWWWWW {:?}", counts);
    }
    // and then find the most distant point
    let mut max_dist: i64 = -1;
    let mut max_idx: usize = 0;
    for (i, (_, count)) in counts.iter().enumerate() {
        let dist: i64 = (count - avg).abs();
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }
    //(max_idx, max_val)
    &counts[max_idx]
}

// can take out Debug trait later
fn brute<
    G: Generate<I> + std::fmt::Display,
    I: 'static + std::fmt::Display + Clone + std::fmt::Debug + std::marker::Send + std::cmp::Ord,
    B: tui::backend::Backend,
>(
    path: &str,
    gen: &mut G,
    get_inst_count: fn(&str, &Input) -> i64,
    terminal: &mut tui::terminal::Terminal<B>,
) {
    loop {
        let size = terminal.size().unwrap();
        let n_workers = 8;
        let mut num_jobs: i64 = 0;
        let mut results: Vec<(I, i64)> = Vec::new();

        let graph: Vec<(String, u64)>;
        let mut graph2: Vec<(&str, u64)> = Vec::new();
        let pool = ThreadPool::new(n_workers);
        let (tx, rx) = channel();

        for inp_pair in gen.by_ref() {
            num_jobs += 1;
            let tx = tx.clone();
            let test = String::from(path);
            pool.execute(move || {
                let inp = inp_pair.1;
                let inst_count = get_inst_count(&test, &inp);
                trace!("inst_count: {:?}", inst_count);
                let _ = tx.send((inp_pair.0, inst_count));
            });
        }
        let mut min: u64 = std::i64::MAX as u64;
        for _ in 0..num_jobs {
            let tmp = rx.recv().unwrap();
            if (tmp.1 as u64) < min {
                min = tmp.1 as u64;
            }
            results.push(tmp);
            // graph2.push((&mut graph.last().unwrap().0, tmp.1 as u64));
        }
        results.sort();
        graph = results
            .iter()
            .map(|s| (format!("{}", s.0), s.1 as u64))
            .collect();
        if !graph.is_empty() {
            terminal
                .draw(|mut f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints(
                            [Constraint::Percentage(70), Constraint::Percentage(100)].as_ref(),
                        ).split(size);

                    BarChart::default()
                        .block(Block::default().title("Data1").borders(Borders::ALL))
                        .data({
                            graph2 = graph
                                .iter()
                                .map(|s| {
                                    let aaaaaa = s.1 - min;
                                    (&*s.0, aaaaaa)
                                }).collect::<Vec<(&str, u64)>>();
                            &graph2
                        }).bar_width(2)
                        .style(Style::default().fg(Color::Yellow))
                        .value_style(Style::default().fg(Color::Black).bg(Color::Yellow))
                        .render(&mut f, chunks[0]);
                    TuiLoggerWidget::default()
                        .block(
                            Block::default()
                                .title("Independent Tui Logger View")
                                .title_style(Style::default().fg(Color::White).bg(Color::Black))
                                .border_style(Style::default().fg(Color::White).bg(Color::Black))
                                .borders(Borders::ALL),
                        ).style(Style::default().fg(Color::White))
                        .render(&mut f, chunks[1]);
                }).unwrap();
        }
        // artificial delay to help see gui
        let stdin = io::stdin();
        for evt in stdin.keys() {
            match evt {
                Ok(Key::Char('q')) => panic!("quitting"),
                _ => break,
            }
        }
        let good_idx = find_outlier(results.as_slice());
        if !gen.update(&good_idx.0) {
            break;
        }
    }
}

fn main() {
    init_logger(LevelFilter::Trace).unwrap();

    // Set default level for unknown targets to Trace
    set_default_level(LevelFilter::Info);

    let matches = App::new("B7")
        .version("0.1.0")
        .arg(
            Arg::with_name("binary")
                .help("Binary to brute force input for")
                .index(1)
                .required(true),
        ).get_matches();

    let path = matches.value_of("binary").unwrap();

    // Set default level for unknown targets to Trace
    let stdout = io::stdout().into_raw_mode().unwrap();
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.hide_cursor().unwrap();

    let mut argcgen = ArgcGenerator::new(0, 51);
    brute(path, &mut argcgen, get_inst_count_perf, &mut terminal);
    let argc = argcgen.get_length();
    let mut argvlengen = ArgvLenGenerator::new(argc, 0, 51);
    brute(path, &mut argvlengen, get_inst_count_perf, &mut terminal);
    let argvlens = argvlengen.get_lengths();

    let mut argvgen = ArgvGenerator::new(argc, argvlens, 0x20, 0x7e);
    brute(path, &mut argvgen, get_inst_count_perf, &mut terminal);
    //let argvlens = argvlengen.get_lengths();

    let mut lgen = StdinLenGenerator::new(0, 51);
    brute(path, &mut lgen, get_inst_count_perf, &mut terminal);
    let stdinlen = lgen.get_length();
    // TODO: We should have a good way of configuring the range
    let mut gen = StdinCharGenerator::new(stdinlen, 0x20, 0x7e);

    brute(path, &mut gen, get_inst_count_perf, &mut terminal);
    info!("Successfully Generated: A{}A", gen);
    println!("Successfully Generated: A{}A", gen);
}
