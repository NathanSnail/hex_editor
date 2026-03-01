use std::{cell::Cell, collections::HashMap, ops::Range};

use thiserror::Error;
use zerocopy::FromBytes;

#[derive(Error, Debug)]
pub enum CursorError {
    #[error("cursor position out of range")]
    OutOfBounds,
}

#[derive(Copy, Clone, Default, Hash, PartialEq, Eq)]
pub struct SectionID(usize);

pub struct Ranged<T> {
    section: SectionID,
    range: Range<usize>,
    value: T,
}

impl<T> Ranged<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Ranged<U> {
        Ranged {
            section: self.section,
            range: self.range.clone(),
            value: f(self.value),
        }
    }
}

#[derive(Clone)]
pub struct Section {
    cursor: Cell<usize>,
    bytes: Box<[u8]>,
    id: SectionID,
}

impl Section {
    pub fn read(&self, amount: usize) -> Option<Ranged<&[u8]>> {
        let pos = self.get_cursor();
        let end = pos + amount;
        self.set_cursor(end).ok()?;
        let range = pos..end;
        let value = &self.bytes[range.clone()];
        Some(Ranged {
            section: self.id,
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

    pub fn new(bytes: Box<[u8]>, id: SectionID) -> Self {
        Self {
            bytes: bytes,
            cursor: 0.into(),
            id: id,
        }
    }
}

#[derive(Default, Clone)]
pub struct SectionRegistry {
    next_id: SectionID,
    sections: HashMap<SectionID, Section>,
}

impl SectionRegistry {
    pub fn new_section(&mut self, bytes: Box<[u8]>) -> &Section {
        let id = self.next_id;
        self.next_id.0 += 1;
        self.sections.insert(id, Section::new(bytes, id));
        self.sections
            .get(&id)
            .expect("Inserted section should exist")
    }

    pub fn get_section(&self, id: SectionID) -> Option<&Section> {
        self.sections.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use zerocopy::U32;
    use zerocopy::byteorder::{BE, LE};

    use super::*;

    #[test]
    fn read_bytes() {
        let mut section_registry = SectionRegistry::default();
        let section = section_registry
            .new_section(Box::new([0x00, 0x00, 0x01, 0x23, 0xff, 0x01, 0x00, 0x00]));
        let first: u32 = section.read_cast::<U32<BE>>().unwrap().into();
        assert_eq!(first, 0x0123);
        let second: u32 = section.read_cast::<U32<LE>>().unwrap().into();
        assert_eq!(second, 0x01ff);
        assert_eq!(section.read_cast::<U32<LE>>(), None)
    }
}
