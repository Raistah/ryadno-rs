use std::{collections::HashMap, fmt::Debug};

use rkyv::{
    Archive, Deserialize, Serialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
};
use serde_json::Value;

use crate::fields::{Field, text_input::TextField};

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Form<T: Field + Archive + Debug + Eq + PartialEq> {
    pub schema: Vec<T>,
    pub uuid: String,
    pub update_endpoint: String,
    pub data: Option<ValueWrapper>,
}

pub struct FormBuilder {}

impl FormBuilder {
    // pub fn new() -> Self {
    //     todo!()
    // }

    // pub fn build() -> Form {
    //     todo!()
    // }
}

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum FieldTypes {
    Text(TextField),
}

impl FieldTypes {
    fn as_field(&self) -> &impl Field {
        match self {
            Self::Text(v) => v,
        }
    }
}

impl Field for FieldTypes {
    fn after_update(
        &mut self,
        value: serde_json::Value,
        old_value: serde_json::Value,
        from_context: &FormContext,
    ) {
        match self {
            Self::Text(v) => v.after_update(value, old_value, from_context),
        };
    }

    fn to_html(
        &self,
        mjenv: &minijinja::Environment<'_>,
        state_path: String,
        value: Option<serde_json::Value>,
        from_context: &FormContext,
    ) -> Result<String, minijinja::Error> {
        self.as_field()
            .to_html(mjenv, state_path, value, from_context)
    }

    fn validate(&self, value: serde_json::Value) -> Result<(), Vec<(String, String)>> {
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.as_field().is_live()
    }

    fn get_name(&self) -> &str {
        self.as_field().get_name()
    }

    fn get_uuid(&self) -> &str {
        self.as_field().get_uuid()
    }
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

#[derive(Debug, PartialEq, Eq)]
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
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Self::Archived::serialize_from_str::<_>(self.0.to_string().as_str(), serializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<ValueWrapper, D> for ArchivedString {
    fn deserialize(&self, _: &mut D) -> Result<ValueWrapper, D::Error> {
        let string = self.to_string();
        let value: serde_json::Value = serde_json::from_str(string.as_str()).unwrap();
        Ok(ValueWrapper(value))
    }
}

#[cfg(test)]
mod test {
    use rkyv::{from_bytes, rancor, to_bytes};

    use super::*;

    #[test]
    fn test_rkyv_with_generic() {
        let form = Form {
            schema: vec![FieldTypes::Text(TextField::make("text".to_string()))],
            update_endpoint: "".to_string(),
            uuid: "".to_string(),
            data: None,
        };

        let bytes = to_bytes::<rancor::Error>(&form).unwrap();
        let restored_form: Form<FieldTypes> =
            from_bytes::<Form<FieldTypes>, rancor::Error>(&bytes).unwrap();
        assert_eq!(form, restored_form);
    }
}
