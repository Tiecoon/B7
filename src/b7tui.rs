use crate::generators::GenItem;
use crate::generators::Input;
use log::LevelFilter;
use std::fs::File;
use std::io;
use std::io::Read;
use std::time::Duration;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{BarChart, Block, Borders, Paragraph, SelectableList, Tabs, Text, Widget};
use tui::Terminal;
use tui_logger::*;

//structs to help opranize the tabs
pub struct TabsState {
    pub titles: Vec<String>,
    pub index: usize,
}

impl TabsState {
    pub fn new(titles: Vec<String>) -> TabsState {
        TabsState { titles, index: 0 }
    }
    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
    }
}

struct App {
    tabs: TabsState,
}

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
    //timout setter
    fn set_timeout(&mut self, timeout: Duration);
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
    app: App,
    repeat: u32,
    timeout: u64,
    options: Vec<String>,
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
        let options = vec!["Repeat".to_string(), "Timeout".to_string()];

        //adding App for multiple tabs
        let app = App {
            tabs: TabsState::new(vec!["Tab1".to_string(), "Tab2".to_string()]),
        };

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
            app,
            repeat: 1,
            timeout: 5,
            options,
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
            None => return,
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
            let app = &mut self.app;
            let options = &mut self.options;
            let selected = self.selected;
            let timeout = Duration::new(self.timeout, 0);
            options[0] = "Repeat: ".to_string() + &self.repeat.to_string();
            options[1] = "Timeout: ".to_string() + &timeout.as_secs().to_string();
            self.terminal
                .draw(|mut f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(0)
                        .constraints(
                            [
                                Constraint::Percentage(8),
                                Constraint::Percentage(50),
                                Constraint::Percentage(20),
                                Constraint::Percentage(15),
                                Constraint::Percentage(8),
                            ]
                            .as_ref(),
                        )
                        .split(size);

                    let chunks2 = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(0)
                        .constraints(
                            [Constraint::Percentage(8), Constraint::Percentage(80)].as_ref(),
                        )
                        .split(size);

                    //tabs widget
                    Tabs::default()
                        .block(Block::default().title("Tabs").borders(Borders::ALL))
                        .titles(&app.tabs.titles)
                        .select(app.tabs.index)
                        .render(&mut f, chunks[0]);

                    match app.tabs.index {
                        0 => {
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
                                .render(&mut f, chunks[1]);

                            // Widget for log levels
                            TuiLoggerWidget::default()
                                .block(
                                    Block::default()
                                        .title("Log Output")
                                        .title_style(
                                            Style::default().fg(Color::White).bg(Color::Black),
                                        )
                                        .border_style(
                                            Style::default().fg(Color::White).bg(Color::Black),
                                        )
                                        .borders(Borders::ALL),
                                )
                                .style(Style::default().fg(Color::White))
                                .render(&mut f, chunks[2]);

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
                                .render(&mut f, chunks[3]);
                            //Adding another box listing commands to be taken
                            let text = [
                                Text::raw(
                                    "right key for next input, left key for previous input\n",
                                ),
                                Text::styled("c", Style::default().modifier(Modifier::Bold)),
                                Text::raw(" to continue, "),
                                Text::styled("q", Style::default().modifier(Modifier::Bold)),
                                Text::raw(" to quit, "),
                                Text::styled("h", Style::default().modifier(Modifier::Bold)),
                                Text::raw(" to convert to hex, "),
                                Text::styled("d", Style::default().modifier(Modifier::Bold)),
                                Text::raw(" to convert to decimal, "),
                                Text::styled("s", Style::default().modifier(Modifier::Bold)),
                                Text::raw(" to convert to string\n"),
                                Text::styled("-", Style::default().modifier(Modifier::Bold)),
                                Text::raw(" and "),
                                Text::styled("=", Style::default().modifier(Modifier::Bold)),
                                Text::raw(" to control barchart spacing, "),
                                Text::styled(",", Style::default().modifier(Modifier::Bold)),
                                Text::raw(" and "),
                                Text::styled(".", Style::default().modifier(Modifier::Bold)),
                                Text::raw(" to switch tabs"),
                            ];
                            //Widget for displaying instructions to the user
                            Paragraph::new(text.iter())
                                .block(Block::default().borders(Borders::NONE))
                                .alignment(Alignment::Center)
                                //.wrap(true)
                                .render(&mut f, chunks[4]);
                        }
                        1 => {
                            SelectableList::default()
                                .block(Block::default().borders(Borders::ALL).title("Options"))
                                .style(Style::default().fg(Color::White))
                                .highlight_style(
                                    Style::default()
                                        .fg(Color::LightGreen)
                                        .modifier(Modifier::Bold),
                                )
                                .items(options)
                                .select(selected)
                                .highlight_symbol(">")
                                .render(&mut f, chunks2[1]);
                        }
                        _ => {}
                    }
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
    fn set_timeout(&mut self, timeout:Duration){
        self.timeout = timeout.as_secs();
    }
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
                        if self.app.tabs.index == 1{
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
                        
                    }
                    Ok(Key::Down) => {
                        if self.app.tabs.index == 1{
                            match self.selected {
                                Some(x) => {
                                    if x < self.options.len() - 1 {
                                        self.selected = Some(x + 1);
                                    }
                                }
                                None => {
                                    self.selected = Some(0);
                                }
                            }
                            self.redraw();
                        }
                    }
                    Ok(Key::Char('=')) => {
                        self.gap += 1;
                    }
                    Ok(Key::Char('-')) => {
                        if self.gap > 0 {
                            self.gap -= 1;
                        }
                    }
                    //switching tabs
                    Ok(Key::Char('.')) => {
                        if self.app.tabs.index < self.app.tabs.titles.len() - 1 {
                            self.app.tabs.index += 1;
                        }
                    }
                    Ok(Key::Char(',')) => {
                        if self.app.tabs.index > 0 {
                            self.app.tabs.index -= 1;
                        }
                    }
                    Ok(Key::Char('\n')) => {
                        if self.app.tabs.index == 1{
                            let mut buffer = String::new();
                            let selection = self.selected;
                            let mut option_index:i32 = -1;
                            match selection {
                                Some(x) => {
                                    option_index = x as i32;
                                }
                                None => {

                                }

                            }
                            for input in io::stdin().keys(){
                                match input{
                                    Ok(Key::Char('\n')) => {
                                        break
                                    }
                                    Ok(Key::Char(x)) =>{
                                        if x.is_digit(10){
                                            buffer.push(x);
                                        }
                                    }
                                    _ => {
                                        
                                    }
                                }
                                match option_index {
                                    0 => {
                                        self.repeat = match buffer.parse::<u32>(){
                                            Ok(x) => {x}
                                            _ => {
                                                buffer = String::new();
                                                0
                                            } 
                                        }
                                    }
                                    1 => {
                                        self.timeout = match buffer.parse::<u64>(){
                                            Ok(x) => {x}
                                            _ => {
                                                buffer = String::new();
                                                0
                                            } 
                                        }
                                    }
                                    _ => {}
                                }
                                self.redraw();
                            }
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
                //allow for bar resizing even if done
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
    fn set_timeout(&mut self,_timeout: Duration){
        ();
    }
    fn wait(&mut self) -> bool {
        true
    }
    fn done(&mut self) -> bool {
        true
    }
}
