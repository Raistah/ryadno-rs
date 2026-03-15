use std::fmt::Display;

use minijinja::context;
use rkyv::{Archive};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{fields::{Field, prepare_value_for_datastar}, form::FormContext, utils::capitalize_first};

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub struct TextField {
	uuid: String,
    name: String,
    label: String,
    live: bool,
    hidden: bool,
    input_type: TextFieldType,
}

impl TextField {
	pub fn make(name: String) -> Self {
        let label = capitalize_first(name.as_str());

        Self {
       		uuid: Uuid::new_v4().to_string(),
            name,
            label,
            live: false,
            hidden: false,
            input_type: TextFieldType::Text,
        }
    }

    pub fn live(mut self) -> Self {
        self.live = true;
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn text(mut self) -> Self {
        self.input_type = TextFieldType::Text;
        self
    }

    pub fn email(mut self) -> Self {
        self.input_type = TextFieldType::Email;
        self
    }

    pub fn tel(mut self) -> Self {
        self.input_type = TextFieldType::Tel;
        self
    }

    pub fn numeric(mut self) -> Self {
        self.input_type = TextFieldType::Numeric;
        self
    }

    pub fn password(mut self) -> Self {
        self.input_type = TextFieldType::Password;
        self
    }

    pub fn url(mut self) -> Self {
        self.input_type = TextFieldType::Url;
        self
    }
}

impl Field for TextField {
    fn after_update(
        &mut self,
        value: serde_json::Value,
        old_value: serde_json::Value,
        from_context: &FormContext,
    ) {
        // TODO: update self based on new value, form context and other modifiers field have
    }

    fn to_html(
        &self,
        mjenv: &minijinja::Environment<'_>,
        state_path: String,
        value: Option<serde_json::Value>,
        from_context: &FormContext,
    ) -> Result<String, minijinja::Error> {
        let value = match value {
            None => "null".to_string(),
            Some(v) => prepare_value_for_datastar(&v)
        };

        mjenv
            .get_template("ryadno/fields/text-input.jinja")?
            .render(context! {
            	uuid => self.uuid,
                label => self.label,
                name => self.name,
                state_path => state_path,
                hidden => self.hidden,
                input_type => self.input_type.to_string(),
                value => value,
                form_context => from_context
            })
    }

    fn validate(&self, value: serde_json::Value) -> Result<(), Vec<(String, String)>> {
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.live
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_uuid(&self) -> &str {
    	&self.uuid
    }
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub enum TextFieldType {
    Text,
    Email,
    Tel,
    Numeric,
    Password,
    Url,
}

impl Display for TextFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Email => write!(f, "email"),
            Self::Tel => write!(f, "tel"),
            Self::Numeric => write!(f, "number"),
            Self::Password => write!(f, "password"),
            Self::Url => write!(f, "url"),
        }
    }
}
