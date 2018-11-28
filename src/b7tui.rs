use log::LevelFilter;
use std::io;
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
use tui_logger::*;

pub enum UiType {
    ENV,
    Tui,
}

pub trait Ui {
    fn update<
        I: 'static + std::fmt::Display + Clone + std::fmt::Debug + std::marker::Send + std::cmp::Ord,
    >(
        &mut self,
        results: &[(I, i64)],
        min: &u64,
    ) -> bool;
    fn wait(&mut self) -> bool;
    fn done(&mut self) -> bool;
}

pub struct Tui {
    terminal: tui::Terminal<
        tui::backend::TermionBackend<
            termion::screen::AlternateScreen<
                termion::input::MouseTerminal<termion::raw::RawTerminal<std::io::Stdout>>,
            >,
        >,
    >,
    size: tui::layout::Rect,
}

impl Tui {
    pub fn new() -> Tui {
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
        Tui { terminal, size }
    }
}

impl Default for Tui {
    fn default() -> Self {
        Self::new()
    }
}

impl Ui for Tui {
    fn update<
        I: 'static + std::fmt::Display + Clone + std::fmt::Debug + std::marker::Send + std::cmp::Ord,
    >(
        &mut self,
        results: &[(I, i64)],
        min: &u64,
    ) -> bool {
        let size = self.terminal.size().unwrap();
        if self.size != size {
            self.terminal.resize(size).unwrap();
            self.size = size;
        }
        let graph: Vec<(String, u64)>;
        let mut graph2: Vec<(&str, u64)> = Vec::new();
        graph = results
            .iter()
            .map(|s| (format!("{}", s.0), s.1 as u64))
            .collect();
        let size = self.size;
        if !graph.is_empty() {
            self.terminal
                .draw(|mut f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints(
                            [Constraint::Percentage(70), Constraint::Percentage(30)].as_ref(),
                        ).split(size);

                    BarChart::default()
                        .block(Block::default().title("B7").borders(Borders::ALL))
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
        true
    }
    fn wait(&mut self) -> bool {
        let stdin = io::stdin();
        for evt in stdin.keys() {
            match evt {
                Ok(Key::Char('q')) => panic!{"Quitting"},
                Ok(Key::Right) => {
                    break;
                }
                _ => {}
            }
        }
        true
    }
    fn done(&mut self) -> bool {
        let stdin = io::stdin();
        for evt in stdin.keys() {
            match evt {
                Ok(Key::Char('q')) => panic!{"Quitting"},
                Ok(Key::Char('p')) => panic!("Force Closing"),
                _ => {}
            }
        }
        true
    }
}

#[derive(Default)]
pub struct Env;

impl Env {
    pub fn new() -> Env {
        let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
        env_logger::Builder::from_env(env)
            .default_format_timestamp(false)
            .init();
        Env {}
    }
}

impl Ui for Env {
    fn update<
        I: 'static + std::fmt::Display + Clone + std::fmt::Debug + std::marker::Send + std::cmp::Ord,
    >(
        &mut self,
        _results: &[(I, i64)],
        _min: &u64,
    ) -> bool {
        true
    }
    fn wait(&mut self) -> bool {
        true
    }
    fn done(&mut self) -> bool {
        true
    }
}
