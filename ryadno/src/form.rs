use std::{collections::HashMap, fmt::Debug};

use minijinja::context;
use rkyv::{
    Archive, Deserialize, Serialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
};
use serde_json::Value;

use crate::{
    fields::{Field, text_input::TextField},
    structs::data_path::DataPath,
};

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Form<T: Field + Archive + Debug + Eq + PartialEq> {
    pub schema: Vec<T>,
    pub uuid: String,
    pub update_endpoint: String,
    pub data: Option<ValueWrapper>,
}

impl<T> Form<T>
where
    T: Field + Archive + Debug + Eq + PartialEq,
{
    fn to_html(&self, mjenv: &minijinja::Environment<'_>) -> Result<String, minijinja::Error> {
        let mut rendered_fields = String::new();
        let form_context = FormContext {
            update_endpoint: self.update_endpoint.clone(),
            headers: HashMap::new(),
            extra: HashMap::new(),
        };

        match &self.data {
            Some(value) => {
                for field in self.schema.iter() {
                    let state_path = DataPath::from(field.get_name());

                    rendered_fields.push_str(
                        field
                            .to_html(
                                mjenv,
                                state_path.clone(),
                                state_path.find_value(&value.0),
                                &form_context,
                            )?
                            .as_str(),
                    );
                }
            }
            None => {
                for field in self.schema.iter() {
                    rendered_fields.push_str(
                        field
                            .to_html(mjenv, DataPath::from(field.get_name()), None, &form_context)?
                            .as_str(),
                    );
                }
            }
        }

        mjenv.get_template("ryadno/form.jinja")?.render(context! {
            html => rendered_fields
        })
    }
}

macro_rules! register_field_type_enum {
    ($enum_name:ident { $($variant:ident($struct:ty)),* $(,)? }) => {
		#[derive(
			$crate::rkyv::Archive,
			$crate::rkyv::Serialize,
			$crate::rkyv::Deserialize,
			Debug, PartialEq, Eq
		)]
        pub enum $enum_name {
            $($variant($struct)),*
        }

        $(
            impl From<$struct> for $enum_name {
                fn from(v: $struct) -> Self {
                    Self::$variant(v)
                }
            }
        )*

        impl Field for $enum_name {
            fn after_update(
                &mut self,
                value: $crate::serde_json::Value,
                old_value: $crate::serde_json::Value,
                from_context: &FormContext,
            ) {
                match self {
                    $(Self::$variant(v) => v.after_update(value, old_value, from_context)),*
                }
            }

            fn to_html(
                &self,
                mjenv: &$crate::minijinja::Environment<'_>,
                state_path: DataPath,
                value: Option<&$crate::serde_json::Value>,
                from_context: &FormContext,
            ) -> Result<String, $crate::minijinja::Error> {
                match self {
                    $(Self::$variant(v) => v.to_html(mjenv, state_path, value, from_context)),*
                }
            }

            fn validate(&self, value: $crate::serde_json::Value) -> Result<(), Vec<(String, String)>> {
                match self {
                    $(Self::$variant(v) => v.validate(value)),*
                }
            }

            fn is_live(&self) -> bool {
                match self {
                    $(Self::$variant(v) => v.is_live()),*
                }
            }

            fn get_name(&self) -> &str {
                match self {
                    $(Self::$variant(v) => v.get_name()),*
                }
            }

            fn get_uuid(&self) -> &str {
                match self {
                    $(Self::$variant(v) => v.get_uuid()),*
                }
            }
        }
    };
}

register_field_type_enum! {
    FieldTypes {
        Text(TextField),
    }
}

macro_rules! to_bytes {
    ($from:expr) => {
        $crate::rkyv::to_bytes::<$crate::rkyv::rancor::Error>($from)
    };
}

macro_rules! from_bytes {
    ($type:ty, $bytes:expr) => {
        $crate::rkyv::from_bytes::<$type, $crate::rkyv::rancor::Error>($bytes)
    };
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
    use minijinja::{Environment, path_loader};
    use serde_json::json;

    use super::*;

    #[test]
    fn test_rkyv_with_generic() {
        let form = Form {
            schema: vec![
                TextField::make("first_name".to_string()).into(),
                TextField::make("last_name".to_string()).into(),
            ],
            update_endpoint: "".to_string(),
            uuid: "".to_string(),
            data: None,
        };

        let bytes = to_bytes!(&form).unwrap();
        let restored_form = from_bytes!(Form<FieldTypes>, &bytes).unwrap();
        assert_eq!(form, restored_form);
    }

    #[test]
    fn test_form_to_html() {
        let form: Form<FieldTypes> = Form {
            schema: vec![
                TextField::make("first_name".to_string()).into(),
                TextField::make("last_name".to_string()).into(),
            ],
            update_endpoint: "".to_string(),
            uuid: "".to_string(),
            data: Some(ValueWrapper(json!({
                "first_name": "hehe",
                "last_name": "hoho"
            }))),
        };

        let mut env = Environment::new();
        env.set_loader(move |name| {
            let real_name = name.replace("ryadno/", "");
            path_loader("./src/templates")(real_name.as_str())
        });

        let html = form.to_html(&env).unwrap();
        assert!(html.contains("value: 'hehe'"));
        assert!(html.contains("path: 'first_name'"));
        assert!(html.contains("value: 'hoho'"));
        assert!(html.contains("path: 'last_name'"));
    }
}
