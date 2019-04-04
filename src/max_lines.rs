use std::io::{BufRead, BufReader, Read, Result};

#[derive(Debug)]
pub struct MaxLines<B> {
    buf: B,
    max_lines: usize,
}

impl<B: BufRead> MaxLines<B> {
    pub fn single(&mut self) -> Option<Result<String>> {
        let mut buf = String::new();
        match self.buf.read_line(&mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf.ends_with("\n") {
                    buf.pop();
                    if buf.ends_with("\r") {
                        buf.pop();
                    }
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

impl<B: BufRead> Iterator for MaxLines<B> {
    type Item = Vec<Result<String>>;

    fn next(&mut self) -> Option<Vec<Result<String>>> {
        let mut ret = Vec::new();
        let mut i = 0;
        while i < self.max_lines {
            match self.single() {
                None => break,
                Some(s) => ret.push(s),
            }
            i += 1;
        }
        if ret.len() == 0 {
            None
        } else {
            Some(ret)
        }
    }
}

pub trait MaxLinesIterator: Sized {
    fn max_lines(self, max_lines: usize) -> MaxLines<Self>
    where
        Self: Sized,
    {
        MaxLines { buf: self, max_lines }
    }
}

impl<R: Read> MaxLinesIterator for BufReader<R> {}
