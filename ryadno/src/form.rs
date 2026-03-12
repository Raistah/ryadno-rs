use std::collections::HashMap;

use serde::Serialize;

#[derive(Serialize)]
pub struct FormContext {
	pub update_endpoint: String,
	pub headers: HashMap<String, String>,
	pub extra: HashMap<String, String>,
}
