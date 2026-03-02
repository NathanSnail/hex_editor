use std::sync::Arc;

use mlua::{AsChunk, Chunk, ExternalError, Lua};

use crate::section::SectionRegistry;

pub struct ScriptableRegistry {
    pub registry: SectionRegistry,
    lua: Lua,
}

impl ScriptableRegistry {
    fn add_api(lua: &Lua) {
        let greet = lua
            .create_function(|_, name: String| {
                name.parse::<u32>().map_err(ExternalError::into_lua_err)
            })
            .unwrap();

        lua.globals().set("greet", greet).unwrap();
    }

    pub fn new(registry: SectionRegistry) -> Self {
        let lua = Lua::new();
        ScriptableRegistry::add_api(&lua);
        Self { registry, lua }
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
        let scriptable_registry = ScriptableRegistry::new(registry);
        assert_eq!(
            scriptable_registry
                .load("greet('2')")
                .eval::<u32>()
                .unwrap(),
            2
        )
    }
}
