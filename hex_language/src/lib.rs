mod section;

pub fn create_context() -> mlua::Lua {
    let context = mlua::Lua::new();

    let greet = context
        .create_function(|_, name: String| {
            println!("Hello {name}");
            Ok(2)
        })
        .unwrap();
    context.globals().set("greet", greet).unwrap();
    context
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_fn() {
        let context = create_context();
        assert_eq!(context.load("greet('foo')").eval::<u32>().unwrap(), 2)
    }
}
