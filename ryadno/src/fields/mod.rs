pub mod select;
pub mod text_input;

use std::{any::Any, fmt::Debug};

use minijinja::Environment;
use rkyv::Archive;
use serde_json::Value;

use crate::{fields::text_input::TextField, form::FormContext, structs::data_path::DataPath};

pub trait Field: Archive + Debug + Eq + PartialEq {
    fn after_update(
        &mut self,
        value: Value,
        old_value: Value,
        from_context: &FormContext,
        runtime_ctx: Option<&dyn Any>,
    );
    fn to_html(
        &self,
        mjenv: &Environment<'_>,
        state_path: DataPath,
        value: Option<&Value>,
        from_context: &FormContext,
    ) -> Result<String, minijinja::Error>;
    fn validate(&self, value: Value) -> Result<(), Vec<(String, String)>>;
    fn get_name(&self) -> &str;
    fn get_uuid(&self) -> &str;
    fn is_live(&self) -> bool;
}

pub fn prepare_value_for_datastar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Number(v) => format!("{v}"),
        serde_json::Value::String(v) => format!("'{v}'"),
        serde_json::Value::Bool(v) => format!("{v}"),
        serde_json::Value::Array(v) => format!("{}", serde_json::to_string(&v).unwrap()),
        serde_json::Value::Object(v) => format!("{}", serde_json::to_string(&v).unwrap()),
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
                runtime_ctx: Option<&dyn Any>
            ) {
                match self {
                    $(Self::$variant(v) => v.after_update(value, old_value, from_context, runtime_ctx)),*
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
