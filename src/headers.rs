use libc::{c_char, c_int, size_t};
use picohttpparser_sys::phr_parse_headers;
use common::{Error, Result, Status};
use header::PicoHeader;
use interop;

pub struct Headers<'buf, 'headers: 'buf> {
    pub headers: &'headers mut [PicoHeader],
    parsed: ParsedHeaders<'buf>,
}

impl<'buf, 'headers> Headers<'buf, 'headers> {
    #[inline]
    pub fn new(buf: &'buf [u8], headers: &'headers mut [PicoHeader]) -> Headers<'buf, 'headers> {
        let len = headers.len();
        Headers {
            headers: headers,
            parsed: ParsedHeaders::new(buf, len),
        }
    }

    pub fn parse(&mut self, last_len: usize) -> Result<Status> {
        unsafe {
            self.parsed.return_code = phr_parse_headers(self.parsed.buf.as_ptr() as *const c_char,
                                                        self.parsed.buf.len(),
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

struct ParsedHeaders<'buf> {
    buf: &'buf [u8],
    num_headers: size_t,
    return_code: c_int,
}

impl<'buf> ParsedHeaders<'buf> {
    #[inline]
    fn new(buf: &'buf [u8], num_headers: usize) -> Self {
        ParsedHeaders {
            buf: buf,
            num_headers: num_headers as size_t,
            return_code: -3,
        }
    }
}

#[cfg(test)]
mod tests {
    use common::{Error, Status};
    use header::{Header, PicoHeader};
    use super::*;

    #[test]
    fn parse_headers() {
        let buf = b"Host: example.com\r\n\r\n";
        let mut headers = [PicoHeader::default(); 100];
        let mut parser = Headers::new(buf, &mut headers);

        match parser.parse(0) {
            Ok(Status::Complete(len)) => assert_eq!(buf.len(), len),
            _ => assert!(false),
        };

        assert_eq!(parser.headers.len(), 1);
        assert_eq!(parser.headers[0].name(), "Host");
        assert_eq!(parser.headers[0].value(), b"example.com");
    }

    #[test]
    fn parse_partial_headers() {
        let mut headers = [PicoHeader::default(); 100];
        let mut parser = Headers::new(b"Host: ", &mut headers);

        match parser.parse(0) {
            Ok(Status::Partial) => assert_eq!(0, parser.headers.len()),
            _ => assert!(false),
        };
    }

    #[test]
    fn parse_error_headers() {
        let mut headers = [PicoHeader::default(); 100];
        let mut parsed = Headers::new(b"H\tst: example.com\r\n\r\n", &mut headers);

        match parsed.parse(0) {
            Err(Error::Parse) => assert!(true),
            _ => assert!(false),
        };
    }
}
