pub mod text_input;

use minijinja::Environment;
use serde_json::Value;

use crate::form::FormContext;

pub trait Field {
    fn make(name: String) -> Self;
    fn after_update(&mut self, value: Value, old_value: Value, from_context: &FormContext);
    fn to_html(
        &self,
        mjenv: &Environment<'_>,
        state_path: String,
        value: Option<Value>,
        from_context: &FormContext,
    ) -> Result<String, minijinja::Error>;
    fn validate(&self, value: Value) -> Result<(), Vec<(String, String)>>;
    fn get_name(&self) -> &String;
    fn get_uuid(&self) -> &String;
    fn is_live(&self) -> bool;
}
