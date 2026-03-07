use std::{fmt::Display, path::PathBuf};

use minijinja::context;

use crate::fields::Field;

pub struct TextField {
    name: String,
    live: bool,
    hidden: bool,
    input_type: TextFieldType,
}

impl TextField {
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
    fn make(name: String) -> Self {
        Self {
            name,
            live: false,
            hidden: false,
            input_type: TextFieldType::Text,
        }
    }

    fn prepare_context(&mut self, value: serde_json::Value) -> minijinja::Value {
        // TODO: based on self and value create a context for to_html() method
        context! {}
    }

    fn to_html(&self, mjenv: &minijinja::Environment<'static>) -> Result<String, minijinja::Error> {
        mjenv
            .get_template("ryadno/fields/text-field.jinja")?
            .render(context! {})
    }

    fn validate(&self, value: serde_json::Value) -> Result<(), Vec<(String, String)>> {
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.live
    }

    fn get_name(&self) -> &String {
        &self.name
    }
}

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
