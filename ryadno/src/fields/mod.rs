pub mod select;
pub mod text_input;

use std::{any::Any, fmt::Debug};

use minijinja::Environment;
use rkyv::Archive;
use serde_json::Value;

use crate::{form::FormContext, structs::data_path::DataPath};

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
