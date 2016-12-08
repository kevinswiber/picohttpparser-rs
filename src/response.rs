use libc::{c_char, c_int, size_t};
use std::ptr;
use std::str;
use picohttpparser_sys::phr_parse_response;
use common::{Error, Result, Status};
use header::PicoHeader;
use interop;

pub struct Response<'buf, 'headers: 'buf> {
    pub headers: &'headers mut [PicoHeader],
    parsed: ParsedResponse<'buf>,
}

impl<'buf, 'headers> Response<'buf, 'headers> {
    #[inline]
    pub fn new(buf: &'buf [u8], headers: &'headers mut [PicoHeader]) -> Response<'buf, 'headers> {
        let len = headers.len();
        Response {
            headers: headers,
            parsed: ParsedResponse::new(buf, len),
        }
    }

    pub fn minor_version(&self) -> Option<u8> {
        if self.parsed.version > -1 {
            Some(self.parsed.version as u8)
        } else {
            None
        }
    }

    pub fn status(&self) -> Option<u8> {
        if self.parsed.status > 0 {
            Some(self.parsed.status as u8)
        } else {
            None
        }
    }

    pub fn description(&self) -> Option<&str> {
        if self.parsed.msg != ptr::null() {
            Some(unsafe { str::from_utf8_unchecked(self.parsed.msg_bytes()) })
        } else {
            None
        }
    }

    pub fn parse(&mut self, last_len: usize) -> Result<Status> {
        unsafe {
            self.parsed.return_code = phr_parse_response(self.parsed.buf.as_ptr() as *const c_char,
                                                         self.parsed.buf.len(),
                                                         &mut self.parsed.version,
                                                         &mut self.parsed.status,
                                                         &mut self.parsed.msg,
                                                         &mut self.parsed.msg_len,
                                                         self.headers.as_mut_ptr(),
                                                         &mut self.parsed.num_headers,
                                                         last_len);
        }

        interop::shrink(&mut self.headers, self.parsed.num_headers);

        match self.parsed.return_code {
            len if len > 0 => Ok(Status::Complete(len as usize)),
            -2 => Ok(Status::Partial),
            -1 => Err(Error::Parse),
            _ => unreachable!(), // invalid return code
        }

    }
}

struct ParsedResponse<'buf> {
    buf: &'buf [u8],
    version: c_int,
    status: c_int,
    msg: *const c_char,
    msg_len: size_t,
    num_headers: size_t,
    return_code: c_int,
}

impl<'buf> ParsedResponse<'buf> {
    #[inline]
    fn new(buf: &'buf [u8], num_headers: usize) -> Self {
        ParsedResponse {
            buf: buf,
            version: -1,
            status: 0,
            msg: ptr::null_mut(),
            msg_len: 0,
            num_headers: num_headers as size_t,
            return_code: -3,
        }
    }

    #[inline]
    fn msg_bytes<'a>(&self) -> &'a [u8] {
        interop::slice_from_raw(self.msg, self.msg_len)
    }
}

#[cfg(test)]
mod tests {
    use common::{Error, Status};
    use header::{Header, PicoHeader};
    use super::*;

    #[test]
    fn parse_response() {
        let buf = b"HTTP/1.1 200 OK\r\n\r\n";
        let mut headers = [PicoHeader::default(); 100];
        let mut response = Response::new(buf, &mut headers);

        match response.parse(0) {
            Ok(Status::Complete(len)) => assert_eq!(buf.len(), len),
            _ => assert!(false),
        };

        assert_eq!(response.minor_version().unwrap(), 1);
        assert_eq!(response.status().unwrap(), 200);
        assert_eq!(response.description().unwrap(), "OK");
        assert_eq!(response.headers.len(), 0);
    }

    #[test]
    fn parse_headers() {
        let buf = b"HTTP/1.0 404 Not Found\r\nHost: example.com\r\n\r\n";
        let mut headers = [PicoHeader::default(); 100];
        let mut response = Response::new(buf, &mut headers);

        match response.parse(0) {
            Ok(Status::Complete(len)) => assert_eq!(buf.len(), len),
            _ => assert!(false),
        };

        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.headers[0].name(), "Host");
        assert_eq!(response.headers[0].value(), b"example.com");
    }

    #[test]
    fn parse_partial_response() {
        let mut headers = [PicoHeader::default(); 100];
        let mut response = Response::new(b"HTTP/1.1 200 OK\r\n", &mut headers);

        match response.parse(0) {
            Ok(Status::Partial) => assert_eq!(200, response.status().unwrap()),
            _ => assert!(false),
        };
    }

    #[test]
    fn parse_error_response() {
        let mut headers = [PicoHeader::default(); 100];
        let mut response = Response::new(b"HTTP/1.0 2\t00 OK\r\n\r\n", &mut headers);

        match response.parse(0) {
            Err(Error::Parse) => assert!(true),
            _ => assert!(false),
        };
    }
}
