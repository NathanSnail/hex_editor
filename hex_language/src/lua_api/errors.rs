use std::sync::Arc;

use anyhow::anyhow;
use mlua::Error;

pub fn anyhow_lua(msg: &'static str) -> Error {
    Error::ExternalError(Arc::from(
        Into::<Box<dyn std::error::Error + 'static>>::into(anyhow!(msg)),
    ))
}

pub fn bad_argument(
    fn_name: &'static str,
    pos: usize,
    arg_name: &'static str,
    cause: &'static str,
) -> Error {
    Error::BadArgument {
        to: Some(fn_name.to_owned()),
        pos,
        name: Some(arg_name.to_owned()),
        cause: Arc::new(anyhow_lua(cause)),
    }
}
