mod lua_errors;
use std::sync::{Arc, Mutex};

use mlua::{AsChunk, Chunk, Error, FromLuaMulti, IntoLuaMulti, Lua, MaybeSend, Table};
use zerocopy::{BE, FromBytes, LE, U32};

use crate::{
    lua_api::lua_errors::bad_argument,
    section::{SectionID, SectionRegistry},
};

pub struct ScriptableRegistry {
    pub registry: Arc<Mutex<SectionRegistry>>,
    lua: Lua,
}

fn lua_read_bytes(
    name: &'static str,
    registry: Arc<Mutex<SectionRegistry>>,
    lua: &Lua,
    (section_id, amount): (usize, usize),
) -> Result<Table, Error> {
    // SAFETY: if this wasn't a valid SectionID then we will just return a lua error
    let section_id = unsafe { SectionID::from_usize(section_id) };
    let lock = registry.lock().unwrap();
    let section = lock.get_section(section_id).ok_or_else(|| {
        bad_argument(
            name,
            1,
            "section_id",
            "Not a valid section id in the registry",
        )
    })?;
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
    registry: Arc<Mutex<SectionRegistry>>,
    lua: &Lua,
    section_id: usize,
) -> Result<T, Error> {
    // SAFETY: if this wasn't a valid SectionID then we will just return a lua error
    let section_id = unsafe { SectionID::from_usize(section_id) };
    let lock = registry.lock().unwrap();
    let section = lock.get_section(section_id).ok_or_else(|| {
        bad_argument(
            name,
            1,
            "section_id",
            "Not a valid section id in the registry",
        )
    })?;
    let value = section
        .read_cast::<T>()
        .ok_or_else(|| bad_argument(name, 2, "amount", "Attempted to read outside of bounds"))?;

    Ok(*value.value())
}

impl ScriptableRegistry {
    fn add_fn<A, R, RL>(
        &self,
        name: &'static str,
        func: impl Fn(&'static str, Arc<Mutex<SectionRegistry>>, &Lua, A) -> Result<R, Error>
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
        macro_rules! primitive_read {
            ($zerocopy_ty: ident, $native_ty: ty) => {
                self.add_fn::<_, $zerocopy_ty<LE>, $native_ty>(
                    concat!("read_l", stringify!($native_ty)),
                    lua_read_cast,
                );
                self.add_fn::<_, $zerocopy_ty<BE>, $native_ty>(
                    concat!("read_b", stringify!($native_ty)),
                    lua_read_cast,
                );
            };
        }

        self.add_fn::<_, _, Table>("read_bytes", lua_read_bytes);
        primitive_read!(U32, u32);
    }

    pub fn new(registry: Arc<Mutex<SectionRegistry>>) -> Self {
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
        let scriptable_registry = ScriptableRegistry::new(Arc::new(Mutex::new(registry)));
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let id = scriptable_registry
            .registry
            .lock()
            .unwrap()
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
        let scriptable_registry = ScriptableRegistry::new(Arc::new(Mutex::new(registry)));
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let id = scriptable_registry
            .registry
            .lock()
            .unwrap()
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
}
