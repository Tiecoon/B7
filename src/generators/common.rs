pub type StringType = Vec<u8>;
pub type ArgumentType = Vec<StringType>;

#[derive(Debug, Clone, Default)]
/// Holds the various input the runner is expected to use
pub struct Input {
    pub argc: u32,
    pub argv: ArgumentType,
    pub stdinlen: u32,
    pub stdin: StringType,
    pub mem: Vec<MemInput>,
}

impl Input {
    pub fn new() -> Input {
        Input::default()
    }
    /// Takes in an Input to essentially copy over self
    /// ## example
    /// ```
    /// use crate::b7::generators::Input;
    /// let mut one = Input::new();
    /// let mut two = Input::new();
    /// one.stdin = vec!['A' as u8,'b' as u8];
    /// two.argc = 2;
    /// let new = one.combine(two);
    /// println!("{:?}",new);
    /// ```
    pub fn combine(self, tmp: Input) -> Input {
        let mut res = self.clone();
        if tmp.argv.len() != 0 {
            res.argv = tmp.argv;
        }
        if tmp.argc != 0 {
            res.argc = res.argc;
        }
        if tmp.stdinlen != 0 {
            res.stdinlen = tmp.stdinlen;
        }
        if tmp.stdin.len() != 0 {
            res.stdin = tmp.stdin;
        }
        if tmp.mem.len() != 0 {
            res.mem = tmp.mem;
        }

        res
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// Input to a memory buffer
pub struct MemInput {
    /// Size of memory buffer
    pub size: usize,
    /// Address of memory buffer
    pub addr: usize,
    /// Bytes to load in memory buffer
    pub bytes: StringType,
}
