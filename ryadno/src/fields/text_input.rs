use std::{any::Any, fmt::Display, pin::Pin, sync::Arc};

use linkme::distributed_slice;
use minijinja::context;
use rkyv::Archive;
use uuid::Uuid;

use crate::{
    fields::{BoolValue, Field, LiveType, prepare_value_for_datastar},
    form::{
        ChangePusher, FormContext, RenderRegistryPusher, Update, UpdateType, ValueGetter,
        ValueSetter,
    },
    structs::{data_path::DataPath, error::Error, field_change},
    utils::capitalize_first,
};

#[distributed_slice]
#[linkme(crate = crate::linkme)]
pub static RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES: [(&'static str, TextFieldHiddenClosure)];
pub type TextFieldHiddenClosure = for<'a> fn(
    data_path: Arc<DataPath>,
    form_context: Arc<FormContext>,
    runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
    get: ValueGetter<'a>,
    set: ValueSetter<'a>,
) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub struct TextField {
    uuid: String,
    name: String,
    data_path: Arc<DataPath>,
    label: String,
    live: LiveType,
    hidden: BoolValue,
    is_hidden: bool,
    input_type: TextFieldType,
}

impl TextField {
    pub fn make(name: String) -> Self {
        let label = capitalize_first(name.as_str());

        Self {
            uuid: Uuid::new_v4().to_string(),
            name: name.clone(),
            data_path: Arc::new(DataPath::from(name)),
            label,
            live: LiveType::Static(false),
            hidden: BoolValue::Static(false),
            is_hidden: false,
            input_type: TextFieldType::Text,
        }
    }

    pub fn live(mut self, live_type: LiveType) -> Self {
        self.live = live_type;
        self
    }

    pub fn hidden(mut self, hidden: BoolValue) -> Self {
        self.hidden = hidden;
        self
    }

    pub async fn is_hidden<'a>(
        &mut self,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
    ) -> bool {
        match &self.hidden {
            BoolValue::Static(v) => v.clone(),
            BoolValue::Closure(handler) => {
                match RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES
                    .iter()
                    .find(|closure| closure.0 == handler.as_str())
                {
                    Some(closure) => {
                        (closure.1)(self.get_data_path(), form_context, runtime_ctx, get, set).await
                    }
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

#[async_trait::async_trait]
impl Field for TextField {
    fn get_data_path(&self) -> Arc<DataPath> {
        self.data_path.clone()
    }

    fn set_data_path(&mut self, data_path: Arc<DataPath>) {
        self.data_path = data_path;
    }

    async fn initial_hydration<'a>(
        &mut self,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
    ) {
        self.is_hidden = self
            .is_hidden(form_context, runtime_ctx.clone(), get.clone(), set.clone())
            .await;
    }

    async fn process_update<'a>(
        &mut self,
        update: Arc<Update>,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
        push: RenderRegistryPusher<'a>,
    ) {
        match &update.update {
            UpdateType::Value(v) => {
                set(self.get_data_path(), v.clone()).await;
            }
            UpdateType::Action(_) => (),
        }
        self.after_update(update, form_context, runtime_ctx, get, set, push)
            .await
    }

    async fn after_update<'a>(
        &mut self,
        _: Arc<Update>,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
        push: RenderRegistryPusher<'a>,
    ) {
        self.is_hidden = self
            .is_hidden(form_context, runtime_ctx.clone(), get.clone(), set.clone())
            .await;

        push(self.get_data_path()).await;
    }

    fn to_html(
        &self,
        mjenv: &minijinja::Environment<'_>,
        value: Option<&serde_json::Value>,
        form_context: Arc<FormContext>,
    ) -> Result<String, Error> {
        let value = match value {
            None => "null".to_string(),
            Some(v) => prepare_value_for_datastar(&v),
        };

        Ok(mjenv
            .get_template("ryadno/fields/text-input.jinja")?
            .render(context! {
                uuid => self.uuid,
                label => self.label,
                name => self.name,
                state_path => self.data_path.to_string(),
                hidden => self.is_hidden,
                input_type => self.input_type.to_string(),
                value => value,
                form_context => form_context
            })?)
    }

    fn push_change(
        &self,
        mjenv: &minijinja::Environment<'_>,
        value: Option<&serde_json::Value>,
        form_context: Arc<FormContext>,
        _: &mut Vec<Arc<DataPath>>,
        push: ChangePusher,
    ) {
        let data = self
            .to_html(mjenv, value, form_context)
            .map_err(|v| v.to_string());
        let selector = if data.is_ok() {
            format!(".ryadno-field-{}", self.get_uuid())
        } else {
            format!(".ryadno-field-{} .ryadno-field-error", self.get_uuid())
        };
        push(
            self.get_data_path(),
            field_change::ChangeType::RerenderField {
                selector: selector,
                data: data,
            },
        )
    }

    fn validate(&self, value: serde_json::Value) -> Result<(), Vec<(String, String)>> {
        Ok(())
    }

    fn is_live(&self) -> &LiveType {
        &self.live
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
