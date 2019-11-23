use crate::generators::GenItem;
use crate::generators::Input;
use log::LevelFilter;
use std::fs::File;
use std::io;
use std::io::Read;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{BarChart, Block, Borders, SelectableList, Widget};
use tui::Terminal;
use tui_logger::*;

enum Format {
    Hex,
    String,
    Decimal,
}

/// Trait that all Uis will implement to ensure genericness
pub trait Ui {
    // handle a new ui check
    fn update(&mut self, results: Box<Vec<(i64, (GenItem, Input))>>, min: u64) -> bool;
    // allow gui to pause if user doesn't want to continue
    fn wait(&mut self) -> bool;
    // separate wait to signify all results are calculated
    fn done(&mut self) -> bool;
}

/// struct for Tui-rs implementation
pub struct Tui {
    // TODO probably can be shortened with generics
    terminal: tui::Terminal<
        tui::backend::TermionBackend<
            termion::screen::AlternateScreen<
                termion::input::MouseTerminal<termion::raw::RawTerminal<std::io::Stdout>>,
            >,
        >,
    >,
    size: tui::layout::Rect,
    cache: Vec<(Vec<(u64, u64)>, u64)>,
    numrun: u64,
    currun: u64,
    gap: u16,
    format: Format,
    cont: bool,
    path: Option<String>,
    history: Vec<String>,
    selected: Option<usize>,
}

// constructor
impl Tui {
    pub fn new(path: Option<String>) -> Tui {
        init_logger(LevelFilter::Trace).unwrap();

        // Set default level for unknown targets to Trace
        set_default_level(LevelFilter::Info);
        let stdout = io::stdout().into_raw_mode().unwrap();
        let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.hide_cursor().unwrap();
        let size = terminal.size().unwrap();
        let cache = Vec::new();
        let history = Vec::new();
        Tui {
            terminal,
            size,
            cache,
            numrun: 0,
            currun: 0,
            gap: 0,
            format: Format::Hex,
            cont: false,
            path,
            history,
            selected: None,
        }
    }
    pub fn set_path(&mut self, path: String) {
        self.path = Some(path.to_string());
    }
    pub fn load_cache(&mut self) {
        // Parse out the cache file
        match self.path {
            Some(ref path) => {
                let mut file = match File::open(format!("{}.cache", path)) {
                    Ok(file) => file,
                    Err(_) => return,
                };
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("Could not read cache file.");
                self.history.clear();
                for line in contents.lines() {
                    self.history.push(line.to_string());
                }
                drop(file);
            }
            None => {}
        }
    }
    pub fn redraw(&mut self) -> bool {
        // resize terminal if needed
        let size = self.terminal.size().unwrap();
        if self.size != size {
            self.terminal.resize(size).unwrap();
            self.size = size;
        }
        self.load_cache();
        if !self.cache.is_empty() {
            let history = &self.history;
            let graph = &self.cache[(self.currun - 1) as usize];
            let graph3: Vec<(String, u64)> = graph
                .0
                .iter()
                .map(|s| match self.format {
                    Format::Decimal => (format!("{}", s.0), s.1 as u64),
                    Format::Hex => (format!("{:x}", s.0), s.1 as u64),
                    Format::String => (
                        format!("{}", String::from_utf8_lossy(&[s.0 as u8])),
                        s.1 as u64,
                    ),
                })
                .collect();

            let mut graph2: Vec<(&str, u64)> = Vec::new();
            let gap = self.gap;
            self.terminal
                .draw(|mut f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints(
                            [
                                Constraint::Percentage(60),
                                Constraint::Percentage(25),
                                Constraint::Percentage(15),
                            ]
                            .as_ref(),
                        )
                        .split(size);

                    BarChart::default()
                        .block(Block::default().title("B7").borders(Borders::ALL))
                        .data({
                            // convert String to &str and chop off uneccesary instructions
                            graph2 = graph3
                                .iter()
                                .map(|s| {
                                    let adjusted = s.1 - graph.1;
                                    (&*s.0, adjusted)
                                })
                                .collect::<Vec<(&str, u64)>>();
                            &graph2
                        })
                        .bar_width(2)
                        .style(Style::default().fg(Color::Yellow))
                        .value_style(Style::default().fg(Color::Black).bg(Color::Yellow))
                        .bar_gap(gap)
                        .render(&mut f, chunks[0]);

                    // Widget for log levels
                    TuiLoggerWidget::default()
                        .block(
                            Block::default()
                                .title("Log Output")
                                .title_style(Style::default().fg(Color::White).bg(Color::Black))
                                .border_style(Style::default().fg(Color::White).bg(Color::Black))
                                .borders(Borders::ALL),
                        )
                        .style(Style::default().fg(Color::White))
                        .render(&mut f, chunks[1]);

                    // List widget for cache
                    SelectableList::default()
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Cached Results"),
                        )
                        .items(&history)
                        //.select(self.selected)
                        .style(Style::default().fg(Color::White))
                        .highlight_style(
                            Style::default()
                                .fg(Color::LightGreen)
                                .modifier(Modifier::Bold),
                        )
                        .highlight_symbol(">")
                        .render(&mut f, chunks[2]);
                })
                .unwrap();
        }
        true
    }
}

// default constructor for syntax sugar
impl Default for Tui {
    fn default() -> Self {
        Self::new(None)
    }
}

// implement Tuis Ui trait
impl Ui for Tui {
    /// draw bargraph for new input
    fn update(&mut self, mut results: Box<Vec<(i64, (GenItem, Input))>>, min: u64) -> bool {
        // convertcachefor barchart
        results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // TODO implement multiple formats
        let graph: Vec<(String, u64)>;
        graph = results
            .iter()
            .map(|s| (format!("{}", (s.1).0), s.0 as u64))
            .collect();
        self.cache.push((
            graph
                .iter()
                .map(|s| ((s.0.parse::<u64>().unwrap()), s.1))
                .collect::<Vec<(u64, u64)>>(),
            min,
        ));
        if self.currun == self.numrun {
            self.currun += 1;
        }
        self.numrun += 1;
        let _ = self.redraw();

        true
    }
    // pause for user input before continuing
    fn wait(&mut self) -> bool {
        let stdin = io::stdin();
        if !self.cont {
            for evt in stdin.keys() {
                match evt {
                    Ok(Key::Char('q')) => panic! {"Quitting"},
                    Ok(Key::Char('h')) => self.format = Format::Hex,
                    Ok(Key::Char('d')) => self.format = Format::Decimal,
                    Ok(Key::Char('s')) => self.format = Format::String,
                    Ok(Key::Char('c')) => {
                        self.cont ^= true;
                        if self.cont {
                            break;
                        }
                    }
                    Ok(Key::Right) => {
                        if self.currun < self.numrun {
                            self.currun += 1;
                        } else {
                            break;
                        }
                    }
                    Ok(Key::Left) => {
                        if self.currun > 1 {
                            self.currun -= 1;
                        }
                    }
                    Ok(Key::Up) => {
                        match self.selected {
                            Some(x) => {
                                if x > 0 {
                                    self.selected = Some(x - 1);
                                }
                            }
                            None => {
                                self.selected = Some(0);
                            }
                        }
                        self.redraw();
                    }
                    Ok(Key::Down) => {
                        match self.selected {
                            Some(x) => {
                                if x < self.history.len() {
                                    self.selected = Some(x + 1);
                                }
                            }
                            None => {
                                self.selected = Some(0);
                            }
                        }
                        self.redraw();
                    }
                    Ok(Key::Char('=')) => {
                        self.gap += 1;
                    }
                    Ok(Key::Char('-')) => {
                        if self.gap > 0 {
                            self.gap -= 1;
                        }
                    }
                    _ => {}
                }
                let _ = self.redraw();
            }
        }
        let _ = self.redraw();
        true
    }
    // wait at the end of the program to show results
    fn done(&mut self) -> bool {
        let stdin = io::stdin();
        for evt in stdin.keys() {
            match evt {
                Ok(Key::Char('q')) => panic! {"Quitting"},
                Ok(Key::Char('p')) => panic!("Force Closing"),
                Ok(Key::Char('h')) => self.format = Format::Hex,
                Ok(Key::Char('d')) => self.format = Format::Decimal,
                Ok(Key::Char('s')) => self.format = Format::String,
                Ok(Key::Right) => {
                    if self.currun < self.numrun {
                        self.currun += 1;
                    }
                }
                Ok(Key::Left) => {
                    if self.currun > 1 {
                        self.currun -= 1;
                    }
                }
                Ok(Key::Up) => {
                    match self.selected {
                        Some(x) => {
                            if x > 0 {
                                self.selected = Some(x - 1);
                            }
                        }
                        None => {
                            self.selected = Some(0);
                        }
                    }
                    self.redraw();
                }
                Ok(Key::Down) => {
                    match self.selected {
                        Some(x) => {
                            if x < self.history.len() {
                                self.selected = Some(x + 1);
                            }
                        }
                        None => {
                            self.selected = Some(0);
                        }
                    }
                    self.redraw();
                }
                _ => {}
            }
            let _ = self.redraw();
        }
        let _ = self.redraw();
        true
    }
}

#[derive(Default)]
pub struct Env;

impl Env {
    // initialize the logging
    pub fn new() -> Env {
        let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
        let _ = env_logger::Builder::from_env(env)
            .default_format_timestamp(false)
            .try_init();
        Env {}
    }
}

// default do nothing just let the prints handle it
impl Ui for Env {
    fn update(&mut self, mut _results: Box<Vec<(i64, (GenItem, Input))>>, _min: u64) -> bool {
        true
    }
    fn wait(&mut self) -> bool {
        true
    }
    fn done(&mut self) -> bool {
        true
    }
}
