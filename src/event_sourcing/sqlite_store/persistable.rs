use serde::Serialize;
use serde::de::DeserializeOwned;

pub trait Persistable: Serialize + DeserializeOwned {}

impl<T> Persistable for T where T: Serialize + DeserializeOwned {}
