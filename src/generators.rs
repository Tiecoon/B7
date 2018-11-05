type StringType = Vec<u8>;
type ArgumentType = Vec<StringType>;

#[derive(Debug)]
pub struct Input {
    pub argv: ArgumentType,
    pub stdin: StringType,
}

impl Input {
    pub fn new(argv: ArgumentType, stdin: StringType) -> Input {
        Input {
            argv: argv,
            stdin: stdin,
        }
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
            max: max,
            correct: 0,
        }
    }

    pub fn get_length(&self) -> u32 {
        self.correct
    }
}

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

impl Events for StdinLenGenerator {
    fn on_update(&self) {
        info!("stdin length: {}", self.correct);
    }
}

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
}

impl std::fmt::Display for StdinCharGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.correct.as_slice()))
    }
}

impl StdinCharGenerator {
    pub fn new(padlen: u32) -> StdinCharGenerator {
        StdinCharGenerator {
            padlen: padlen,
            padchr: 0x41,
            prefix: vec![],
            suffix: vec![],
            idx: 0,
            cur: 0,
            correct: vec![],
        }
    }

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

impl Iterator for StdinCharGenerator {
    type Item = (u8, Input);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.padlen || self.cur > 255 {
            return None;
        }
        let chr = self.cur as u8;
        self.cur += 1;
        let mut inp: StringType = Vec::new();
        inp.extend_from_slice(&self.prefix);
        inp.extend_from_slice(&self.correct);
        inp.push(chr);
        inp.extend_from_slice(&self.suffix);
        while inp.len() > self.padlen as usize {
            inp.pop();
        }
        while inp.len() < self.padlen as usize {
            inp.push(self.padchr);
        }
        Some((chr, Input::new(vec![], inp)))
    }
}

impl Events for StdinCharGenerator {
    fn on_update(&self) {
        info!("{}", self);
    }
}

impl Update for StdinCharGenerator {
    type Id = u8;

    fn update(&mut self, chosen: &u8) -> bool {
        self.correct.push(*chosen);
        self.idx += 1;
        self.cur = 0;
        self.on_update();
        self.idx < self.padlen
    }
}
