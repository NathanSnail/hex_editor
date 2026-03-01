pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

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
    fn it_works() {
        let context = create_context();
        assert_eq!(context.load("greet('foo')").eval::<u32>().unwrap(), 2)
    }
}
