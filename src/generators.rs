type StringType = Vec<u8>;
type ArgumentType = Vec<StringType>;

#[derive(Debug)]
pub struct Input {
    pub argv: ArgumentType,
    pub stdin: StringType,
}

impl Input {
    pub fn new(argv: ArgumentType, stdin: StringType) -> Input {
        Input { argv, stdin }
    }
}

/*
 * GENERATORS:
 * the brute forcer will proceed in a sequence of rounds
 * each round is composed of:
 *   collect all inputs to try from the generator
 *   execute program with collected inputs and get inst counts
 *   choose the right input (stats analysis)
 *   notify generator which was chosen
 *     generator updates its internal state
 *     returns true, next round will return next inputs to try
 *     or false if done
 *
 * generators follow this spec:
 *   iteration: should return (Id, Input)
 *     Id is an arbitrary type, an identifier the generator uses to identify the input
 *   update: when brute forcer chooses the (argv, stdin) pair that was best,
 *     it calls update(Id) passing the associated Id of the chosen input
 */

// sub-trait might not be needed...
pub trait Update: Iterator {
    type Id;
    fn update(&mut self, chosen: &Self::Id) -> bool;
}

// Generate trait: has iteration and updating with right Id type
pub trait Generate<T>: Iterator<Item = (T, Input)> + Update<Id = T> {}

pub trait Events {
    fn on_update(&self) {}
}

// a blanket impl: any type T that implements iteration and updating with
// the right types has an (empty) impl for Generate
impl<T: Iterator<Item = (U, Input)> + Update<Id = U>, U> Generate<U> for T {}

/* code for stdin generators */
#[derive(Debug)]
pub struct StdinLenGenerator {
    len: u32,
    max: u32,
    correct: u32,
}

impl std::fmt::Display for StdinLenGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.correct)
    }
}

impl StdinLenGenerator {
    pub fn new(min: u32, max: u32) -> StdinLenGenerator {
        StdinLenGenerator {
            len: min,
            max,
            correct: 0,
        }
    }

    // return the number figured out so far
    pub fn get_length(&self) -> u32 {
        self.correct
    }
}

// implement an Iterator to make bruter nicer
impl Iterator for StdinLenGenerator {
    type Item = (u32, Input);

    fn next(&mut self) -> Option<Self::Item> {
        if self.len > self.max {
            return None;
        }
        let sz = self.len;
        self.len += 1;
        Some((sz, Input::new(vec![], vec![0x41; sz as usize])))
    }
}

// setup hooks for update
impl Events for StdinLenGenerator {
    fn on_update(&self) {
        info!("stdin length: {}", self.correct);
    }
}

// setup hook for length
impl Update for StdinLenGenerator {
    type Id = u32;

    fn update(&mut self, chosen: &u32) -> bool {
        self.correct = *chosen;
        self.on_update();
        false
    }
}

#[derive(Debug)]
pub struct StdinCharGenerator {
    padlen: u32,
    padchr: u8,
    prefix: StringType,
    suffix: StringType,
    idx: u32,
    cur: u16,
    correct: StringType,
    min: u16,
    max: u16,
}

// allowing printing of string in flag
impl std::fmt::Display for StdinCharGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.correct.as_slice()))
    }
}

impl StdinCharGenerator {
    pub fn new(padlen: u32, min: u16, max: u16) -> StdinCharGenerator {
        StdinCharGenerator {
            padlen,
            padchr: 0x41,
            prefix: vec![],
            suffix: vec![],
            idx: 0,
            cur: min,
            correct: vec![],
            min,
            max,
        }
    }

    pub fn new_start(padlen: u32, min: u16, max: u16, start: &[u8]) -> StdinCharGenerator {
        warn!("aaaa {:?}", start);
        StdinCharGenerator {
            padlen: padlen - start.len() as u32,
            padchr: 0x41,
            prefix: start.to_vec(),
            suffix: vec![],
            idx: 0,
            cur: min,
            correct: vec![],
            min,
            max,
        }
    }

    // Decide which character to use for padding
    pub fn set_padchr(&mut self, padchr: u8) {
        self.padchr = padchr;
    }
    pub fn set_prefix(&mut self, prefix: StringType) {
        self.prefix = prefix;
    }
    pub fn set_suffix(&mut self, suffix: StringType) {
        self.suffix = suffix;
    }

    pub fn get_input(&self) -> &StringType {
        &self.correct
    }
}

// nice Iterator wrapper for Bruter
impl Iterator for StdinCharGenerator {
    type Item = (u8, Input);

    fn next(&mut self) -> Option<Self::Item> {
        // check if we have anymore to solve
        if self.idx >= self.padlen || self.cur > 255 || self.cur > self.max {
            return None;
        }
        let chr = self.cur as u8;
        self.cur += 1;
        let mut inp: StringType = Vec::new();
        inp.extend_from_slice(&self.prefix);
        inp.extend_from_slice(&self.correct);
        inp.push(chr);
        inp.extend_from_slice(&self.suffix);
        // add padding to reach the required length
        while inp.len() > self.padlen as usize {
            inp.pop();
        }
        while inp.len() < self.padlen as usize {
            inp.push(self.padchr);
        }
        Some((chr, Input::new(vec![], inp)))
    }
}

// update on Char Generator
impl Events for StdinCharGenerator {
    fn on_update(&self) {
        info!("{}", self);
    }
}

// update hook for stdin
impl Update for StdinCharGenerator {
    type Id = u8;

    fn update(&mut self, chosen: &u8) -> bool {
        self.correct.push(*chosen);
        self.idx += 1;
        self.cur = self.min as u16;
        self.on_update();
        self.idx < self.padlen
    }
}

/* code for argv generators */
#[derive(Debug)]
pub struct ArgcGenerator {
    len: u32,
    max: u32,
    correct: u32,
}

// Make sure it prints correctly
impl std::fmt::Display for ArgcGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.correct)
    }
}

// setup constructor and getters
impl ArgcGenerator {
    pub fn new(min: u32, max: u32) -> ArgcGenerator {
        ArgcGenerator {
            len: min,
            max,
            correct: 0,
        }
    }

    pub fn get_length(&self) -> u32 {
        self.correct
    }
}

// nice Iterator Wrapper for use in bruter
impl Iterator for ArgcGenerator {
    type Item = (u32, Input);

    fn next(&mut self) -> Option<Self::Item> {
        if self.len > self.max {
            return None;
        }
        let sz = self.len;
        self.len += 1;
        Some((sz, Input::new(vec![vec![]; sz as usize], vec![])))
    }
}

// Log event hooks
impl Events for ArgcGenerator {
    fn on_update(&self) {
        info!("argc: {}", self.correct);
    }
}

// argc generator
impl Update for ArgcGenerator {
    type Id = u32;

    fn update(&mut self, chosen: &u32) -> bool {
        self.correct = *chosen;
        self.on_update();
        false
    }
}

#[derive(Debug)]
pub struct ArgvLenGenerator {
    len: u32,
    min: u32,
    max: u32,
    pos: usize,
    argc: u32,
    correct: Vec<u32>,
}

// argv length display
impl std::fmt::Display for ArgvLenGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for val in &self.correct {
            write!(f, "argv {}", val);
        }
        write!(f, "done argv")
    }
}

// argv lengths generator
impl ArgvLenGenerator {
    pub fn new(argc: u32, min: u32, max: u32) -> ArgvLenGenerator {
        ArgvLenGenerator {
            len: min,
            min,
            max,
            pos: 0,
            argc,
            correct: vec![0; argc as usize],
        }
    }

    pub fn get_lengths(&self) -> &Vec<u32> {
        &self.correct
    }
}

// nice iterator wrapper to generate guesses
impl Iterator for ArgvLenGenerator {
    type Item = (u32, Input);

    fn next(&mut self) -> Option<Self::Item> {
        // check if we have any left to solve
        if self.len > self.max {
            return None;
        }
        let sz = self.len;
        self.len += 1;
        let mut argv: ArgumentType = Vec::new();
        // add padding to meet length requirement
        for i in 0..self.argc {
            if i == self.pos as u32 {
                argv.push(vec![0x41; sz as usize]);
            } else {
                argv.push(vec![0x41; self.correct[i as usize] as usize]);
            }
        }
        Some((sz, Input::new(argv, vec![])))
    }
}

// Argv length event hook
impl Events for ArgvLenGenerator {
    fn on_update(&self) {
        for val in &self.correct {
            info!("argv {}", val);
        }
    }
}

// Argv length handle guess
impl Update for ArgvLenGenerator {
    type Id = u32;

    fn update(&mut self, chosen: &u32) -> bool {
        self.correct[self.pos] = *chosen;
        self.pos += 1;

        self.len = self.min;
        self.on_update();
        (self.pos as u32) < self.argc
    }
}

#[derive(Debug)]
pub struct ArgvGenerator {
    len: Vec<u32>,
    padchr: u8,
    idx: u32,
    min: u16,
    max: u16,
    pos: usize,
    argc: u32,
    correct: ArgumentType,
    current: StringType,
    cur: u16,
}

// properly format Argv string
impl std::fmt::Display for ArgvGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for val in &self.correct {
            write!(f, "argv {}", String::from_utf8_lossy(val.as_slice()));
        }
        write!(f, "done argv")
    }
}

// argv constructor
impl ArgvGenerator {
    pub fn new(argc: u32, len: &[u32], min: u16, max: u16) -> ArgvGenerator {
        ArgvGenerator {
            len: len.to_vec(),
            padchr: 0x41,
            idx: 0,
            min,
            max,
            pos: 0,
            argc,
            correct: vec![vec![]; argc as usize],
            current: vec![],
            cur: min,
        }
    }

    pub fn get_argv(&self) -> &ArgumentType {
        &self.correct
    }
}

// argv next guess Iterator
impl Iterator for ArgvGenerator {
    type Item = (u8, Input);

    fn next(&mut self) -> Option<Self::Item> {
        if self.argc == 0 {
            return None;
        }
        if self.len[self.pos] == 0 {
            self.pos += 1;
            if (self.pos as u32) >= self.argc {
                return None;
            }
        }
        let len: u32 = self.len[self.pos];
        if self.idx >= len || self.cur > 255 || self.cur > self.max {
            return None;
        }
        let chr = self.cur as u8;
        self.cur += 1;
        //generate current string
        let mut argv: ArgumentType = Vec::new();
        let mut inp: StringType = Vec::new();
        inp.extend_from_slice(&self.current);
        inp.push(chr);
        while inp.len() > len as usize {
            inp.pop();
        }
        while inp.len() < len as usize {
            inp.push(self.padchr);
        }
        //loop and add the values to guessed argv
        for i in 0..self.argc {
            if i == self.pos as u32 {
                argv.push(inp.clone());
            } else if i < self.pos as u32 {
                argv.push(self.correct[i as usize].clone());
            } else {
                argv.push(vec![self.padchr as u8; self.len[i as usize] as usize]);
            }
        }
        Some((chr, Input::new(argv, vec![])))
    }
}

// hook event handling
impl Events for ArgvGenerator {
    fn on_update(&self) {
        for val in &self.correct {
            info!("argv {}", String::from_utf8_lossy(val.as_slice()));
        }
    }
}

// handle correct guess
impl Update for ArgvGenerator {
    type Id = u8;

    fn update(&mut self, chosen: &u8) -> bool {
        // check if we are at the end
        if (self.pos as u32) >= self.argc {
            return (self.pos as u32) < self.argc;
        }

        // push new guess to state
        self.correct[self.pos].push(*chosen);
        self.current.push(*chosen);
        self.cur = self.min as u16;
        self.idx += 1;
        self.on_update();

        if self.idx >= self.len[self.pos] {
            self.pos += 1;
            self.idx = 0;
        }

        (self.pos as u32) < self.argc
    }
}
