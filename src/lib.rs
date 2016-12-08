extern crate libc;
extern crate picohttpparser_sys;

mod request;
mod interop;

use std::result;
use std::str;
use picohttpparser_sys::phr_header;

pub use self::request::Request;

pub enum Status {
    Complete(usize),
    Partial,
}

pub enum Error {
    Parse,
}

pub type Result<T> = result::Result<T, Error>;
pub type PicoHeader = phr_header;

pub trait Header {
    fn name(&self) -> &str;
    fn value(&self) -> &[u8];
}

impl Header for PicoHeader {
    fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(interop::slice_from_raw(self.name, self.name_len)) }
    }

    fn value(&self) -> &[u8] {
        interop::slice_from_raw(self.value, self.value_len)
    }
}
