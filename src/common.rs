use std::result;

pub enum Status {
    Complete(usize),
    Partial,
}

pub enum Error {
    Parse,
}

pub type Result<T> = result::Result<T, Error>;
