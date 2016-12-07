extern crate libc;
extern crate picohttpparser_sys;

use libc::{c_char, c_int, size_t};
use std::mem;
use std::ptr;
use std::result;
use std::slice;
use std::str;
use picohttpparser_sys::{phr_header, phr_parse_request};

pub enum Status {
    Complete(usize),
    Partial,
}

pub enum Error {
    Parse,
}

type Result<T> = result::Result<T, Error>;
pub type PicoHeader = phr_header;

trait Header {
    fn name(&self) -> &str;
    fn value(&self) -> &[u8];
}

impl Header for PicoHeader {
    fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(slice_from_raw(self.name, self.name_len)) }
    }

    fn value(&self) -> &[u8] {
        slice_from_raw(self.value, self.value_len)
    }
}

pub struct Request<'buf, 'headers: 'buf> {
    pub headers: &'headers mut [PicoHeader],
    parsed: ParsedRequest<'buf>,
}

impl<'buf, 'headers> Request<'buf, 'headers> {
    #[inline]
    pub fn new(buf: &'buf [u8], headers: &'headers mut [PicoHeader]) -> Request<'buf, 'headers> {
        let len = headers.len();
        Request {
            headers: headers,
            parsed: ParsedRequest::new(buf, len),
        }
    }

    pub fn method(&self) -> Option<&str> {
        if self.parsed.method != ptr::null() {
            Some(unsafe { str::from_utf8_unchecked(self.parsed.method_bytes()) })
        } else {
            None
        }
    }

    pub fn path(&self) -> Option<&str> {
        if self.parsed.method != ptr::null() {
            Some(unsafe { str::from_utf8_unchecked(self.parsed.path_bytes()) })
        } else {
            None
        }
    }

    pub fn minor_version(&self) -> Option<u8> {
        if self.parsed.version > -1 {
            Some(self.parsed.version as u8)
        } else {
            None
        }
    }

    pub fn parse(&mut self, last_len: usize) -> Result<Status> {
        unsafe {
            self.parsed.return_code = phr_parse_request(self.parsed.buf.as_ptr() as *const c_char,
                                                        self.parsed.buf.len(),
                                                        &mut self.parsed.method,
                                                        &mut self.parsed.method_len,
                                                        &mut self.parsed.path,
                                                        &mut self.parsed.path_len,
                                                        &mut self.parsed.version,
                                                        self.headers.as_mut_ptr(),
                                                        &mut self.parsed.num_headers,
                                                        last_len);
        }

        shrink(&mut self.headers, self.parsed.num_headers);

        match self.parsed.return_code {
            len if len > 0 => Ok(Status::Complete(len as usize)),
            -2 => Ok(Status::Partial),
            -1 => Err(Error::Parse),
            _ => unreachable!(), // invalid return code
        }

    }
}

struct ParsedRequest<'buf> {
    buf: &'buf [u8],
    num_headers: size_t,
    method: *const c_char,
    method_len: size_t,
    path: *const c_char,
    path_len: size_t,
    version: c_int,
    return_code: c_int,
}

impl<'buf> ParsedRequest<'buf> {
    #[inline]
    fn new(buf: &'buf [u8], num_headers: usize) -> Self {
        ParsedRequest {
            buf: buf,
            num_headers: num_headers as size_t,
            method: ptr::null_mut(),
            method_len: 0,
            path: ptr::null_mut(),
            path_len: 0,
            version: -1,
            return_code: -3,
        }
    }

    #[inline]
    fn method_bytes<'a>(&self) -> &'a [u8] {
        slice_from_raw(self.method, self.method_len)
    }

    #[inline]
    fn path_bytes<'a>(&self) -> &'a [u8] {
        slice_from_raw(self.path, self.path_len)
    }
}

#[inline]
fn slice_from_raw<'a>(pointer: *const c_char, len: size_t) -> &'a [u8] {
    unsafe { mem::transmute(slice::from_raw_parts(pointer, len)) }
}

#[inline]
fn shrink<T>(slice: &mut &mut [T], len: usize) {
    debug_assert!(slice.len() >= len);
    let ptr = slice.as_mut_ptr();
    *slice = unsafe { slice::from_raw_parts_mut(ptr, len) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::Header;

    #[test]
    fn parse_request() {
        let buf = b"GET / HTTP/1.0\r\n\r\n";
        let mut headers = [PicoHeader::default(); 100];
        let mut request = Request::new(buf, &mut headers);

        match request.parse(0) {
            Ok(Status::Complete(len)) => assert_eq!(buf.len(), len),
            _ => assert!(false),
        };

        assert_eq!(request.headers.len(), 0);
    }

    #[test]
    fn parse_headers() {
        let buf = b"GET / HTTP/1.0\r\nHost: example.com\r\n\r\n";
        let mut headers = [PicoHeader::default(); 100];
        let mut request = Request::new(buf, &mut headers);

        match request.parse(0) {
            Ok(Status::Complete(len)) => assert_eq!(buf.len(), len),
            _ => assert!(false),
        };

        assert_eq!(request.headers.len(), 1);
        assert_eq!(request.headers[0].name(), "Host");
        assert_eq!(request.headers[0].value(), b"example.com");
    }

    #[test]
    fn parse_partial_request() {
        let mut headers = [PicoHeader::default(); 100];
        let mut request = Request::new(b"GET / HTTP/1.0\r\n", &mut headers);

        match request.parse(0) {
            Ok(Status::Partial) => assert_eq!("/", request.path().unwrap()),
            _ => assert!(false),
        };
    }

    #[test]
    fn parse_error_request() {
        let mut headers = [PicoHeader::default(); 100];
        let mut request = Request::new(b"G\tT / HTTP/1.0\r\n\r\n", &mut headers);

        match request.parse(0) {
            Err(Error::Parse) => assert!(true),
            _ => assert!(false),
        };
    }
}
