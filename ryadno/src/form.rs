use std::collections::HashMap;

use rkyv::{
    Archive, Deserialize, Serialize, access, rancor::{self, Fallible, Source}, ser::{Allocator, Writer}, string::{ArchivedString, StringResolver}, to_bytes
};
use serde_json::Value;

use crate::{fields::text_input::TextField, structs::error::Error};

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct Form {
    pub schema: Vec<FieldTypes>,
    pub uuid: String,
    pub update_endpoint: String,
    pub data: Option<ValueWrapper>,
}

impl Form {
	pub fn serialize(&self) -> Result<Vec<u8>, Error> {
		match to_bytes::<rancor::Error>(self) {
			Ok(v) => Ok(v.into_vec()),
			Err(err) => Err(Error::Rkyv(err))
		}
	}

	pub fn from_bytes(bytes: Vec<u8>) -> Result<Form, Error> {
		match access::<ArchivedForm, rancor::Error>(&bytes) {
			Ok(v) => rkyv::deserialize::<Form, rancor::Error>(v).map_err(|err| Error::Rkyv(err)),
			Err(err) => Err(Error::Rkyv(err))
		}
	}
}

pub struct FormBuilder {}

impl FormBuilder {
    pub fn new() -> Self {
        todo!()
    }

    pub fn build() -> Form {
        todo!()
    }
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum FieldTypes {
    Text(TextField),
}

#[derive(serde::Serialize)]
pub struct FormContext {
    pub update_endpoint: String,
    pub headers: HashMap<String, String>,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdatePayload {
    pub uuid: String,
    pub path: String,
    pub value: Value,
}

#[derive(Debug)]
pub struct ValueWrapper(pub Value);

impl Archive for ValueWrapper {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        let s = self.0.to_string();
        ArchivedString::resolve_from_str(&s, resolver, out);
    }
}

impl<S> Serialize<S> for ValueWrapper
where
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Self::Archived::serialize_from_str::<_>(
            self.0.to_string().as_str(),
            serializer,
        )
    }
}

impl<D: Fallible + ?Sized> Deserialize<ValueWrapper, D> for ArchivedString {
    fn deserialize(&self, _: &mut D) -> Result<ValueWrapper, D::Error> {
    	let string = self.to_string();
     	let value: serde_json::Value = serde_json::from_str(string.as_str()).unwrap();
        Ok(ValueWrapper(value))
    }
}
