use std::{cell::Cell, ops::Range};

use thiserror::Error;
use zerocopy::FromBytes;

#[derive(Error, Debug)]
pub enum CursorError {
    #[error("cursor position out of range")]
    OutOfBounds,
}

pub struct Ranged<'section, T> {
    section: &'section Section<'section>,
    range: Range<usize>,
    value: T,
}

pub struct Section<'bytes> {
    cursor: Cell<usize>,
    bytes: &'bytes [u8],
}

impl<'bytes> Section<'bytes> {
    pub fn read(&self, amount: usize) -> Option<Ranged<'_, &[u8]>> {
        let pos = self.get_cursor();
        let end = pos + amount;
        self.set_cursor(end).ok()?;
        let range = pos..end;
        let value = &self.bytes[range.clone()];
        Some(Ranged {
            section: &self,
            range,
            value,
        })
    }

    pub fn read_cast<T: FromBytes>(&self) -> Option<T> {
        self.read(size_of::<T>()).map(|x| {
            T::read_from_bytes(x.value)
                .expect("Ranged data should always be the correct size for T")
        })
    }

    pub fn get_cursor(&self) -> usize {
        self.cursor.get()
    }

    pub fn set_cursor(&self, pos: usize) -> Result<(), CursorError> {
        if pos > self.bytes.len() {
            Err(CursorError::OutOfBounds)
        } else {
            self.cursor.set(pos);
            Ok(())
        }
    }

    pub fn new(bytes: &'bytes [u8]) -> Self {
        Self {
            bytes,
            cursor: 0.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use zerocopy::U32;
    use zerocopy::byteorder::{BE, LE};

    use super::*;

    #[test]
    fn read_bytes() {
        let section = Section::new(&[0x00, 0x00, 0x01, 0x23, 0xff, 0x01, 0x00, 0x00]);
        let first: u32 = section.read_cast::<U32<BE>>().unwrap().into();
        assert_eq!(first, 0x0123);
        let second: u32 = section.read_cast::<U32<LE>>().unwrap().into();
        assert_eq!(second, 0x01ff);
        assert_eq!(section.read_cast::<U32<LE>>(), None)
    }
}
