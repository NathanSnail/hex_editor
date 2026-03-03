mod lua_errors;
use std::sync::Arc;

use mlua::{AsChunk, Chunk, ExternalError, Lua};

use crate::{
    lua_api::lua_errors::{anyhow_lua, bad_argument},
    section::{SectionID, SectionRegistry},
};

pub struct ScriptableRegistry {
    pub registry: Arc<SectionRegistry>,
    lua: Lua,
}

impl ScriptableRegistry {
    fn add_api(&self) {
        let greet = self
            .lua
            .create_function(|_, name: String| {
                name.parse::<u32>().map_err(ExternalError::into_lua_err)
            })
            .unwrap();

        let read_bytes_name = "read_bytes";
        let borrowed = self.registry.clone();
        let read_bytes = self
            .lua
            .create_function(move |_, args: (usize, usize)| {
                let section_id = args.0;
                let amount = args.1;
                // SAFETY: if this wasn't a valid SectionID then we will just return a lua error
                let section = unsafe {
                    let section_id = SectionID::from_usize(section_id);
                    borrowed.get_section(section_id).ok_or_else(|| {
                        bad_argument(
                            read_bytes_name,
                            1,
                            "section_id",
                            "Not a valid section id in the registry",
                        )
                    })?
                };
                section
                    .read(amount)
                    .ok_or_else(|| anyhow_lua("Attempted to read outside of bounds"))?;
                Ok(())
            })
            .unwrap();

        self.lua.globals().set("read_bytes", greet).unwrap();
        self.lua.globals().set("greet", read_bytes).unwrap();
    }

    pub fn new(registry: Arc<SectionRegistry>) -> Self {
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
    use super::*;

    #[test]
    fn example_fn() {
        let registry = SectionRegistry::default();
        let scriptable_registry = ScriptableRegistry::new(Arc::new(registry));
        assert_eq!(
            scriptable_registry
                .load("greet('2')")
                .eval::<u32>()
                .unwrap(),
            2
        )
    }
}
