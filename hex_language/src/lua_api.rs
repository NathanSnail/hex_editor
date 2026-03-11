mod errors;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use mlua::{AsChunk, Chunk, Error, FromLuaMulti, IntoLuaMulti, Lua, MaybeSend, Table};
use paste::paste;
use zerocopy::{BE, FromBytes, I16, I32, I64, I128, LE, U16, U32, U64, U128};

use crate::{
    lua_api::errors::bad_argument,
    section::{Section, SectionID, SectionRegistry},
};

pub struct ScriptableRegistry {
    pub registry: Rc<RefCell<SectionRegistry>>,
    lua: Lua,
}

fn get_section<'a>(
    section_id: usize,
    registry: &'a Rc<RefCell<SectionRegistry>>,
    fn_name: &'static str,
    pos: usize,
) -> Result<Ref<'a, Section>, Error> {
    // SAFETY: if this wasn't a valid SectionID then we will just return a lua error
    let section_id = unsafe { SectionID::from_usize(section_id) };
    Ref::filter_map(registry.borrow(), |reg| reg.get_section(section_id)).map_err(|_| {
        bad_argument(
            fn_name,
            pos,
            "section_id",
            "Not a valid section id in the registry",
        )
    })
}

fn lua_read_bytes(
    name: &'static str,
    registry: Rc<RefCell<SectionRegistry>>,
    lua: &Lua,
    (section_id, amount): (usize, usize),
) -> Result<Table, Error> {
    let section = get_section(section_id, &registry, name, 1)?;
    let bytes = section
        .read(amount)
        .ok_or_else(|| bad_argument(name, 2, "amount", "Attempted to read outside of bounds"))?;

    let table = lua.create_table().unwrap();
    for (k, v) in bytes.value().iter().enumerate() {
        table.set(k + 1, *v).unwrap();
    }
    Ok(table)
}

fn lua_read_cast<T: FromBytes + Copy>(
    name: &'static str,
    registry: Rc<RefCell<SectionRegistry>>,
    _: &Lua,
    section_id: usize,
) -> Result<T, Error> {
    let section = get_section(section_id, &registry, name, 1)?;
    let value = section
        .read_cast::<T>()
        .ok_or_else(|| bad_argument(name, 2, "amount", "Attempted to read outside of bounds"))?;

    Ok(*value.value())
}

fn lua_write_bytes(
    name: &'static str,
    registry: Rc<RefCell<SectionRegistry>>,
    lua: &Lua,
    (section_id, bytes): (usize, Table),
) -> Result<Table, Error> {
    todo!()
}

impl ScriptableRegistry {
    fn add_fn<A, R, RL>(
        &self,
        name: &'static str,
        func: impl Fn(&'static str, Rc<RefCell<SectionRegistry>>, &Lua, A) -> Result<R, Error>
        + MaybeSend
        + 'static,
    ) where
        A: FromLuaMulti,
        R: Into<RL>,
        RL: IntoLuaMulti,
    {
        let registry = self.registry.clone();
        let lua_func = self
            .lua
            .create_function(move |lua, args| {
                func(name, registry.clone(), lua, args).map(Into::into)
            })
            .unwrap();

        self.lua.globals().set(name, lua_func).unwrap();
    }

    fn add_api(&self) {
        macro_rules! primitive_type_read {
            ($bits: literal, $signedness: ident, $endianness: ident) => {
                self.add_fn::<_, paste! {[<$signedness $bits>]<[<$endianness E>]> }, paste! {[<$signedness:lower $bits>]}>(
                    paste! {concat!("read_", stringify!([<$endianness:lower $signedness:lower>]), stringify!($bits))},
                    lua_read_cast,
                )
            };
        }
        macro_rules! primitive_bit_read {
            ($bits: literal) => {
                primitive_type_read!($bits, U, L);
                primitive_type_read!($bits, I, L);
                primitive_type_read!($bits, U, B);
                primitive_type_read!($bits, I, B);
            };
        }

        self.add_fn::<_, _, Table>("read_bytes", lua_read_bytes);
        self.add_fn::<_, i8, i8>("read_i8", lua_read_cast);
        self.add_fn::<_, u8, u8>("read_u8", lua_read_cast);
        primitive_bit_read!(16);
        primitive_bit_read!(32);
        primitive_bit_read!(64);
        primitive_bit_read!(128);
    }

    pub fn new(registry: Rc<RefCell<SectionRegistry>>) -> Self {
        let lua = Lua::new();
        let registry = Self { registry, lua };
        registry.add_api();
        registry
    }

    pub fn load<'a>(&self, src: impl AsChunk + 'a) -> Chunk<'a> {
        self.lua.load(src)
    }
}

#[cfg(test)]
mod tests {
    use mlua::{Function, Table};

    use super::*;

    #[test]
    fn lua_read_bytes() {
        let registry = SectionRegistry::default();
        let scriptable_registry = ScriptableRegistry::new(Rc::new(RefCell::new(registry)));
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let id = scriptable_registry
            .registry
            .borrow_mut()
            .new_section(Box::new(bytes))
            .id()
            .to_usize();
        let table = scriptable_registry
            .load("return function(n) return read_bytes(n, 4) end")
            .eval::<Function>()
            .unwrap()
            .call::<Table>(id)
            .unwrap();
        // TODO: find out how to ipairs from rust
        for i in 0..4 {
            assert_eq!(table.get::<u8>(i + 1).unwrap(), bytes[i]);
        }
    }
    #[test]
    fn lua_read_lu32() {
        let registry = SectionRegistry::default();
        let scriptable_registry = ScriptableRegistry::new(Rc::new(RefCell::new(registry)));
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let id = scriptable_registry
            .registry
            .borrow_mut()
            .new_section(Box::new(bytes))
            .id()
            .to_usize();
        let as_u32 = scriptable_registry
            .load("return function(n) return read_lu32(n) end")
            .eval::<Function>()
            .unwrap()
            .call::<u32>(id)
            .unwrap();
        assert_eq!(as_u32, 0x04030201);
    }

    #[test]
    fn lua_read_bi16() {
        let registry = SectionRegistry::default();
        let scriptable_registry = ScriptableRegistry::new(Rc::new(RefCell::new(registry)));
        let bytes = [0x80, 0x10];
        let id = scriptable_registry
            .registry
            .borrow_mut()
            .new_section(Box::new(bytes))
            .id()
            .to_usize();
        let as_u32 = scriptable_registry
            .load("return function(n) return read_bi16(n) end")
            .eval::<Function>()
            .unwrap()
            .call::<i16>(id)
            .unwrap();
        assert_eq!(as_u32, (0x10i64 - (1 << 15)).try_into().unwrap());
    }

    #[test]
    fn lua_read_u8() {
        let registry = SectionRegistry::default();
        let scriptable_registry = ScriptableRegistry::new(Rc::new(RefCell::new(registry)));
        let bytes = [0x80];
        let id = scriptable_registry
            .registry
            .borrow_mut()
            .new_section(Box::new(bytes))
            .id()
            .to_usize();
        let as_u32 = scriptable_registry
            .load("return function(n) return read_u8(n) end")
            .eval::<Function>()
            .unwrap()
            .call::<u8>(id)
            .unwrap();
        assert_eq!(as_u32, bytes[0]);
    }
}
