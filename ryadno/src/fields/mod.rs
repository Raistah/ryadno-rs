pub mod text_input;

use minijinja::Environment;
use serde_json::Value;

pub trait Field {
	fn make(name: String) -> Self;
	fn prepare_context(&mut self, value: Value) -> minijinja::Value;
	fn to_html(&self, mjenv: &Environment<'_>, context: minijinja::Value) -> Result<String, minijinja::Error>;
	fn validate(&self, value: Value) -> Result<(), Vec<(String, String)>>;
	fn get_name(&self) -> &String;
	fn is_live(&self) -> bool;
}
