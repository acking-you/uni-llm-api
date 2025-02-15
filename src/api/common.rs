use serde::{Deserialize, Deserializer};

use super::uni_ollama::message::Role;

pub(crate) fn null_to_default<'de, D, T>(deserde: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    let opt = Option::deserialize(deserde)?;
    Ok(opt.unwrap_or_default())
}

pub(crate) const fn default_chat_resp_role() -> Role {
    Role::Assistant
}
