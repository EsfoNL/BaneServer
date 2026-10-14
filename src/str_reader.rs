use std::ops::Range;

use tokio::io::{AsyncRead, AsyncReadExt};

pub struct StrReader<R: AsyncRead + Unpin> {
    buf: Box<[u8]>,
    reader: R,
    leftover: Range<usize>,
}

pub const DEFAULT_BUF_SIZE: usize = 256;
impl<R: AsyncRead + Unpin> StrReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            buf: Box::new([0; DEFAULT_BUF_SIZE]),
            reader,
            leftover: 0..0,
        }
    }

    pub fn new_with_capacity(reader: R, capacity: usize) -> Self {
        Self {
            buf: Box::from(vec![0; capacity]),
            reader,
            leftover: 0..0,
        }
    }

    /// returns an empty string on eof
    /// to match [std::io::Read::read] behaviour
    pub async fn read(&mut self) -> std::io::Result<&str> {
        self.buf.copy_within(self.leftover.clone(), 0); // copy the leftovers to the front
        let leftover_len = self.leftover.len();
        let read = self.reader.read(&mut self.buf[leftover_len..]).await? + leftover_len;
        self.leftover = 0..0;
        match str::from_utf8(&self.buf[..read]) {
            Ok(s) => Ok(s),
            Err(e) => {
                if e.valid_up_to() == 0 {
                    return Err(std::io::ErrorKind::InvalidData.into()); // garbage
                }
                // SAFETY: check docs of Utf8Error
                self.leftover = e.valid_up_to()..read;
                let s = unsafe { str::from_utf8_unchecked(&self.buf[..e.valid_up_to()]) };
                Ok(s)
            }
        }
    }
}
