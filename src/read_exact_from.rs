use tokio_io::AsyncRead;
use futures::{Future, Poll};
use std::io;
use std::mem;

#[derive(Debug)]
pub struct ReadExact<A, T> {
    state: State<A, T>,
}

#[derive(Debug)]
enum State<A, T> {
    Reading {
        a: A,
        buf: T,
        pos: usize,
        initial_pos: usize,
    },
    Empty,
}

pub fn read_exact_from<A, T>(a: A, buf: T, pos: usize) -> ReadExact<A, T>
where
    A: AsyncRead,
    T: AsMut<[u8]>,
{
    ReadExact {
        state: State::Reading {
            a,
            buf,
            pos,
            initial_pos: pos,
        },
    }
}

fn eof() -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "early eof")
}

impl<A, T> Future for ReadExact<A, T>
where
    A: AsyncRead,
    T: AsMut<[u8]>,
{
    type Item = (A, T);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(A, T), io::Error> {
        match self.state {
            State::Reading {
                ref mut a,
                ref mut buf,
                ref mut pos,
                ..
            } => {
                let buf = buf.as_mut();
                while *pos < buf.len() {
                    let n = try_nb!(a.read(&mut buf[*pos..]));
                    *pos += n;
                    if n == 0 {
                        return Err(eof());
                    }
                }
            }
            State::Empty => panic!("poll a ReadExact after it's done"),
        }

        match mem::replace(&mut self.state, State::Empty) {
            State::Reading { a, buf, .. } => Ok((a, buf).into()),
            State::Empty => panic!(),
        }
    }
}

impl<A, T> ReadExact<A, T> {
    pub fn try_abort(&mut self) -> Option<(A, T)> {
        if let State::Reading {
            a,
            buf,
            pos,
            initial_pos,
        } = mem::replace(&mut self.state, State::Empty)
        {
            if pos == initial_pos {
                Some((a, buf))
            } else {
                None
            }
        } else {
            None
        }
    }
}
