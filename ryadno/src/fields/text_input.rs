use std::{any::Any, fmt::Display, pin::Pin, sync::Arc};

use futures::future::BoxFuture;
use linkme::distributed_slice;
use minijinja::context;
use rkyv::Archive;
use uuid::Uuid;

use crate::{
    fields::{BoolValue, Field, prepare_value_for_datastar},
    form::FormContext,
    structs::data_path::DataPath,
    utils::capitalize_first,
};

#[distributed_slice]
pub static RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES: [(&'static str, TextFieldHiddenClosure)];
pub type TextFieldHiddenClosure = for<'a> fn(
    &TextField,
    value: Option<&serde_json::Value>,
    from_context: &FormContext,
    runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
    get: Arc<dyn Fn(DataPath) -> BoxFuture<'a, Option<serde_json::Value>> + Sync + Send + 'a>,
    set: Arc<dyn Fn(DataPath, serde_json::Value) -> BoxFuture<'a, Option<DataPath>> + Sync + Send + 'a>,
) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub struct TextField {
    uuid: String,
    name: String,
    label: String,
    live: bool,
    hidden: BoolValue,
    is_hidden: bool,
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
            hidden: BoolValue::Static(false),
            is_hidden: false,
            input_type: TextFieldType::Text,
        }
    }

    pub fn live(mut self) -> Self {
        self.live = true;
        self
    }

    pub fn hidden(mut self, hidden: BoolValue) -> Self {
        self.hidden = hidden;
        self
    }

    pub async fn is_hidden<'a>(
        &mut self,
        value: Option<&serde_json::Value>,
        from_context: &FormContext,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: Arc<dyn Fn(DataPath) -> BoxFuture<'a, Option<serde_json::Value>> + Sync + Send + 'a>,
        set: Arc<dyn Fn(DataPath, serde_json::Value) -> BoxFuture<'a, Option<DataPath>> + Sync + Send + 'a>,
    ) -> bool {
        match &self.hidden {
            BoolValue::Static(v) => v.clone(),
            BoolValue::Closure(handler) => {
                match RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES
                    .iter()
                    .find(|closure| closure.0 == handler.as_str())
                {
                    Some(closure) => (closure.1)(self, value, from_context, runtime_ctx, get, set).await,
                    None => false,
                }
            }
        }
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
    async fn initial_hydration<'a>(
        &mut self,
        value: Option<&serde_json::Value>,
        form_context: &FormContext,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: Arc<dyn Fn(DataPath) -> BoxFuture<'a, Option<serde_json::Value>> + Sync + Send + 'a>,
        set: Arc<dyn Fn(DataPath, serde_json::Value) -> BoxFuture<'a, Option<DataPath>> + Sync + Send + 'a>,
    ) {
        self.is_hidden = self.is_hidden(value, form_context, runtime_ctx, get, set).await;
    }

    async fn after_update(
        &mut self,
        value: serde_json::Value,
        old_value: serde_json::Value,
        from_context: &FormContext,
        runtime_ctx: Option<&dyn Any>,
    ) {
        // TODO: update self based on new value, form context and other modifiers field have
    }

    fn to_html(
        &self,
        mjenv: &minijinja::Environment<'_>,
        state_path: DataPath,
        value: Option<&serde_json::Value>,
        from_context: &FormContext,
    ) -> Result<String, minijinja::Error> {
        let value = match value {
            None => "null".to_string(),
            Some(v) => prepare_value_for_datastar(&v),
        };

        mjenv
            .get_template("ryadno/fields/text-input.jinja")?
            .render(context! {
                uuid => self.uuid,
                label => self.label,
                name => self.name,
                state_path => state_path.to_string(),
                hidden => self.is_hidden,
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
