use crate::generators::Events;
use crate::generators::Update;

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
    pub fn new(input: Input, min: u16, max: u16) -> StdinCharGenerator {
        StdinCharGenerator {
            padlen: input.stdinlen,
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

    pub fn new_start(input: Input, min: u16, max: u16, start: &[u8]) -> StdinCharGenerator {
        warn!("{:?}", start);
        StdinCharGenerator {
            padlen: input.stdinlen,
            padchr: 0x41,
            prefix: start.to_vec(),
            suffix: vec![],
            idx: start.len() as u32,
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
        let mut res = Input::new();
        res.stdin = inp;
        Some((chr, res))
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
