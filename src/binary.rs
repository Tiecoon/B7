#[derive(Debug)]
pub struct Binary {
    path: String
}

impl Binary {
    pub fn new(path: &str) -> Binary {
        Binary {
            path: path.to_string()
        }
    }
}
