#[derive(Debug)]
pub struct Binary {
    path: String,
}

// holds the path to the binary possibly expanded later
impl Binary {
    pub fn new(path: &str) -> Binary {
        Binary {
            path: path.to_string(),
        }
    }
}
