use std::{
    cell::Cell,
    collections::HashMap,
    ops::{Deref, Range},
};

use thiserror::Error;
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum CursorError {
    #[error("cursor position out of range")]
    OutOfBounds,
}

#[derive(Copy, Clone, Default, Hash, PartialEq, Eq, Debug)]
pub struct SectionID(usize);

#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn range(&self) -> &Range<usize> {
        &self.range
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn section(&self) -> SectionID {
        self.section
    }
}

impl<T> Deref for Ranged<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

#[derive(Clone)]
pub struct Section {
    cursor: Cell<usize>,
    bytes: Box<[u8]>,
    id: SectionID,
}

impl Section {
    /// Reads `amount` bytes from the current cursor position and advances the cursor, does nothing
    /// if it cannot be read
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

    /// Reads a `T` from the current cursor position and advances the cursor, does nothing if it
    /// cannot be read
    pub fn read_cast<T: FromBytes>(&self) -> Option<Ranged<T>> {
        self.read(size_of::<T>()).map(|x| {
            x.map(|value| {
                T::read_from_bytes(value)
                    .expect("Ranged data should always be the correct size for T")
            })
        })
    }

    /// Writes `bytes` at the current cursor and advances the cursor, does nothing if it cannot be
    /// written
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), CursorError> {
        let start = self.get_cursor();
        self.set_cursor(start + bytes.len())?;
        let end = self.get_cursor();
        self.bytes[start..end].copy_from_slice(bytes);
        Ok(())
    }

    /// Writes `value` at the current cursor and advances the cursor, does nothing if it cannot be
    /// written
    pub fn write_cast<T: IntoBytes + Immutable>(&mut self, value: T) -> Result<(), CursorError> {
        let bytes = value.as_bytes();
        self.write(bytes)
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
            bytes,
            cursor: 0.into(),
            id,
        }
    }

    pub fn id(&self) -> SectionID {
        self.id
    }
}

#[derive(Default, Clone)]
pub struct SectionRegistry {
    next_id: SectionID,
    sections: HashMap<SectionID, Section>,
}

impl SectionRegistry {
    pub fn new_section(&mut self, bytes: Box<[u8]>) -> &mut Section {
        let id = self.next_id;
        self.next_id.0 += 1;
        self.sections.insert(id, Section::new(bytes, id));
        self.sections
            .get_mut(&id)
            .expect("Inserted section should exist")
    }

    pub fn get_section(&self, id: SectionID) -> Option<&Section> {
        self.sections.get(&id)
    }

    pub fn get_section_mut(&mut self, id: SectionID) -> Option<&mut Section> {
        self.sections.get_mut(&id)
    }
}

#[cfg(test)]
mod tests {
    use zerocopy::byteorder::{BE, LE};
    use zerocopy::{U16, U32};

    use super::*;

    #[test]
    fn read_bytes() {
        let mut section_registry = SectionRegistry::default();
        let section = section_registry
            .new_section(Box::new([0x00, 0x00, 0x01, 0x23, 0xff, 0x01, 0x00, 0x00]));
        let first: u32 = (*section.read_cast::<U32<BE>>().unwrap()).into();
        assert_eq!(first, 0x0123);
        let second: u32 = (*section.read_cast::<U32<LE>>().unwrap()).into();
        assert_eq!(second, 0x01ff);
        assert_eq!(section.read_cast::<U32<LE>>(), None)
    }

    #[test]
    fn write_bytes() {
        let mut section_registry = SectionRegistry::default();
        let section = section_registry.new_section(Box::new([0x00, 0x00, 0x00, 0x00]));

        let value: U32<LE> = 0x1234.into();
        section.write_cast(value).unwrap();
        assert_eq!(section.write_cast(1234), Err(CursorError::OutOfBounds));

        section.set_cursor(0).unwrap();
        assert_eq!(value, *section.read_cast::<U32<LE>>().unwrap());
    }

    #[test]
    fn read_ranged() {
        let mut section_registry = SectionRegistry::default();
        let section = section_registry.new_section(Box::new([0x00, 0x00, 0x00, 0x00]));

        assert_eq!(*section.read_cast::<U16<LE>>().unwrap().range(), 0..2);
        assert_eq!(*section.read_cast::<U16<LE>>().unwrap().range(), 2..4);
    }
}
