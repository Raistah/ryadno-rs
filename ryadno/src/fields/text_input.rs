use std::{any::Any, fmt::Display, pin::Pin, sync::Arc};

use linkme::distributed_slice;
use minijinja::context;
use regex::Regex;
use rkyv::Archive;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    fields::{BoolValue, Field, LiveType, prepare_value_for_datastar},
    form::{
        ChangePusher, FormContext, RenderRegistryPusher, Update, UpdateType, ValueGetter,
        ValueSetter,
    },
    structs::{
        data_path::DataPath,
        error::Error,
        field_change,
        form_content::FormContent,
        update_event::{Debounce, Throttle, UpdateBehavior, UpdateEvent},
        validation::{ExpectedType, RYADNO_FIELDS_VALIDATION_CLOUSRES, ValidationRule},
    },
    utils::{
        capitalize_first, is_valid_email, is_valid_hex_color, is_valid_ip, is_valid_mac_address,
        is_valid_ulid, is_valid_url, is_valid_uuid, json::is_valid_json,
    },
};

#[distributed_slice]
#[linkme(crate = crate::linkme)]
pub static RYADNO_FIELDS_TEXTFIELD_BOOL_CLOUSRES: [(&'static str, TextFieldHiddenClosure)];
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
    live: LiveType, // add some params for update call(on event, debounce)
    update_behavior: UpdateBehavior,
    label: String,
    hide_label: bool,
    placeholder: Option<String>,
    hidden: BoolValue,
    is_hidden: bool,
    input_type: TextFieldType,
    default_value: Option<String>,
    disabled: BoolValue,
    is_disabled: bool,
    required: BoolValue,
    is_required: bool,
    autofocus: BoolValue,
    is_autofocus: bool,
    column_span: u8,
    validation_rules: Option<Vec<ValidationRule>>,

    above_label_content: FormContent,
    above_label_content_cached: Option<String>,
    before_label_content: FormContent,
    before_label_content_cached: Option<String>,
    after_label_content: FormContent,
    after_label_content_cached: Option<String>,
    below_label_content: FormContent,
    below_label_content_cached: Option<String>,
    before_input_content: FormContent,
    before_input_content_cached: Option<String>,
    after_input_content: FormContent,
    after_input_content_cached: Option<String>,
    above_error_content: FormContent,
    above_error_content_cached: Option<String>,
    below_error_content: FormContent,
    below_error_content_cached: Option<String>,
    below_content: FormContent,
    below_content_cached: Option<String>,
}

impl TextField {
    pub fn make(name: String) -> Self {
        let label = capitalize_first(name.as_str()).replace("_", " ");

        Self {
            uuid: Uuid::new_v4().to_string(),
            name: name.clone(),
            data_path: Arc::new(DataPath::from(name)),
            live: LiveType::Static(false),
            update_behavior: UpdateBehavior::default_with_event(UpdateEvent::Input),
            label,
            hide_label: false,
            placeholder: None,
            hidden: BoolValue::Static(false),
            is_hidden: false,
            input_type: TextFieldType::Text,
            default_value: None,
            disabled: BoolValue::Static(false),
            is_disabled: false,
            required: BoolValue::Static(false),
            is_required: false,
            autofocus: BoolValue::Static(false),
            is_autofocus: false,
            column_span: 1,
            validation_rules: None,

            above_label_content: FormContent::default(),
            above_label_content_cached: None,
            before_label_content: FormContent::default(),
            before_label_content_cached: None,
            after_label_content: FormContent::default(),
            after_label_content_cached: None,
            below_label_content: FormContent::default(),
            below_label_content_cached: None,
            before_input_content: FormContent::default(),
            before_input_content_cached: None,
            after_input_content: FormContent::default(),
            after_input_content_cached: None,
            above_error_content: FormContent::default(),
            above_error_content_cached: None,
            below_error_content: FormContent::default(),
            below_error_content_cached: None,
            below_content: FormContent::default(),
            below_content_cached: None,
        }
    }

    pub fn live(mut self, live_type: LiveType) -> Self {
        self.live = live_type;
        self
    }

    pub fn update_event(mut self, value: UpdateEvent) -> Self {
        self.update_behavior.event = value;
        self
    }

    pub fn debounce(mut self, value: Debounce) -> Self {
        self.update_behavior.debounce = Some(value);
        self
    }

    pub fn throttle(mut self, value: Throttle) -> Self {
        self.update_behavior.throttle = Some(value);
        self
    }

    fn update_field<T: PartialEq>(field: &mut T, new_value: T) -> bool {
        if *field != new_value {
            *field = new_value;
            true
        } else {
            false
        }
    }

    pub fn hidden(mut self, value: BoolValue) -> Self {
        self.hidden = value;
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
                match RYADNO_FIELDS_TEXTFIELD_BOOL_CLOUSRES
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

    pub fn label(mut self, value: String) -> Self {
        self.label = value;
        self
    }

    pub fn hide_label(mut self) -> Self {
        self.hide_label = true;
        self
    }

    pub fn placeholder(mut self, value: String) -> Self {
        self.placeholder = Some(value);
        self
    }

    /// Default value assigns during initial_hydration.
    /// If the value of a field depends on the value of another field, it is possible that the target field has not yet been assigned its default value.
    pub fn default_value(mut self, value: String) -> Self {
        self.default_value = Some(value);
        self
    }

    pub fn disabled(mut self, value: BoolValue) -> Self {
        self.disabled = value;
        self
    }

    pub async fn is_disabled<'a>(
        &mut self,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
    ) -> bool {
        match &self.disabled {
            BoolValue::Static(v) => v.clone(),
            BoolValue::Closure(handler) => {
                match RYADNO_FIELDS_TEXTFIELD_BOOL_CLOUSRES
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

    pub fn required(mut self, value: BoolValue) -> Self {
        self.required = value;
        self
    }

    pub async fn is_required<'a>(
        &mut self,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
    ) -> bool {
        match &self.required {
            BoolValue::Static(v) => v.clone(),
            BoolValue::Closure(handler) => {
                match RYADNO_FIELDS_TEXTFIELD_BOOL_CLOUSRES
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

    pub fn autofocus(mut self, value: BoolValue) -> Self {
        self.autofocus = value;
        self
    }

    pub async fn is_autofocus<'a>(
        &mut self,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
    ) -> bool {
        match &self.autofocus {
            BoolValue::Static(v) => v.clone(),
            BoolValue::Closure(handler) => {
                match RYADNO_FIELDS_TEXTFIELD_BOOL_CLOUSRES
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

    pub fn column_span(mut self, value: u8) -> Self {
        self.column_span = value;
        self
    }

    pub fn above_label_content(mut self, value: FormContent) -> Self {
        self.above_label_content = value;
        self
    }

    pub fn before_label_content(mut self, value: FormContent) -> Self {
        self.before_label_content = value;
        self
    }

    pub fn after_label_content(mut self, value: FormContent) -> Self {
        self.after_label_content = value;
        self
    }

    pub fn below_label_content(mut self, value: FormContent) -> Self {
        self.below_label_content = value;
        self
    }

    pub fn before_input_content(mut self, value: FormContent) -> Self {
        self.before_input_content = value;
        self
    }

    pub fn after_input_content(mut self, value: FormContent) -> Self {
        self.after_input_content = value;
        self
    }

    pub fn above_error_content(mut self, value: FormContent) -> Self {
        self.above_error_content = value;
        self
    }

    pub fn below_error_content(mut self, value: FormContent) -> Self {
        self.below_error_content = value;
        self
    }

    pub fn below_content(mut self, value: FormContent) -> Self {
        self.below_content = value;
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
        if let Some(default_value) = &self.default_value
            && get(self.get_data_path()).await == None
        {
            set(self.get_data_path(), Value::String(default_value.clone())).await;
            self.default_value = None;
        }

        self.is_hidden = self
            .is_hidden(
                form_context.clone(),
                runtime_ctx.clone(),
                get.clone(),
                set.clone(),
            )
            .await;

        self.is_disabled = self
            .is_disabled(
                form_context.clone(),
                runtime_ctx.clone(),
                get.clone(),
                set.clone(),
            )
            .await;

        self.is_required = self
            .is_required(
                form_context.clone(),
                runtime_ctx.clone(),
                get.clone(),
                set.clone(),
            )
            .await;

        self.is_autofocus = self
            .is_autofocus(form_context, runtime_ctx.clone(), get.clone(), set.clone())
            .await;

        self.above_label_content_cached = self.above_label_content.to_html();
        self.before_label_content_cached = self.before_label_content.to_html();
        self.after_label_content_cached = self.after_label_content.to_html();
        self.below_label_content_cached = self.below_label_content.to_html();
        self.before_input_content_cached = self.before_input_content.to_html();
        self.after_input_content_cached = self.after_input_content.to_html();
        self.above_error_content_cached = self.above_error_content.to_html();
        self.below_error_content_cached = self.below_error_content.to_html();
        self.below_content_cached = self.below_content.to_html();
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
        let new_is_hidden = self
            .is_hidden(
                form_context.clone(),
                runtime_ctx.clone(),
                get.clone(),
                set.clone(),
            )
            .await;
        if Self::update_field(&mut self.is_hidden, new_is_hidden) {
            push(self.get_data_path()).await;
        }

        let new_is_disabled = self
            .is_disabled(
                form_context.clone(),
                runtime_ctx.clone(),
                get.clone(),
                set.clone(),
            )
            .await;
        if Self::update_field(&mut self.is_disabled, new_is_disabled) {
            push(self.get_data_path()).await;
        }

        let new_is_required = self
            .is_required(
                form_context.clone(),
                runtime_ctx.clone(),
                get.clone(),
                set.clone(),
            )
            .await;
        if Self::update_field(&mut self.is_required, new_is_required) {
            push(self.get_data_path()).await;
        }

        let new_is_autofocus = self
            .is_autofocus(form_context, runtime_ctx.clone(), get.clone(), set.clone())
            .await;
        if Self::update_field(&mut self.is_autofocus, new_is_autofocus) {
            push(self.get_data_path()).await;
        }
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
                name => self.name,
                value => value,
                form_context => form_context,

                autofocus => self.is_autofocus,
                column_span => self.get_column_span(),
                disabled => self.is_disabled,
                hidden => self.is_hidden,
                hide_label => self.hide_label,
                input_type => self.input_type.to_string(),
                label => self.label,
                placeholder => self.placeholder,
                required => self.is_required,
                state_path => self.data_path.to_string(),
                update_behavior => self.update_behavior.to_string(),

                above_label_content => self.above_label_content_cached,
                above_label_content_align => self.above_label_content.align,
                before_label_content => self.before_label_content_cached,
                before_label_content_align => self.before_label_content.align,
                after_label_content => self.after_label_content_cached,
                after_label_content_align => self.after_label_content.align,
                below_label_content => self.below_label_content_cached,
                below_label_content_align => self.below_label_content.align,
                before_input_content => self.before_input_content_cached,
                before_input_content_align => self.before_input_content.align,
                after_input_content => self.after_input_content_cached,
                after_input_content_align => self.after_input_content.align,
                above_error_content => self.above_error_content_cached,
                above_error_content_align => self.above_error_content.align,
                below_error_content => self.below_error_content_cached,
                below_error_content_align => self.below_error_content.align,
                below_content => self.below_content_cached,
                below_content_align => self.below_content.align,
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

    async fn validate<'a>(
        &self,
        value: Option<&serde_json::Value>,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
    ) -> Result<(), Vec<ValidationRule>> {
        match &self.validation_rules {
            Some(validation_rules) => {
                let mut errors: Vec<ValidationRule> = Vec::new();

                for validation_rule in validation_rules.iter() {
                    match validation_rule {
                        ValidationRule::Required => {
                            if value.is_none() {
                                errors.push(validation_rule.clone())
                            }
                        }
                        ValidationRule::Is(expected_type) => match (expected_type, value) {
                            (ExpectedType::Null, Some(Value::Null))
                            | (ExpectedType::Bool, Some(Value::Bool(_)))
                            | (ExpectedType::Number, Some(Value::Number(_)))
                            | (ExpectedType::String, Some(Value::String(_)))
                            | (ExpectedType::Array, Some(Value::Array(_)))
                            | (ExpectedType::Object, Some(Value::Object(_))) => {
                                continue;
                            }
                            _ => errors.push(validation_rule.clone()),
                        },
                        ValidationRule::Same(data_path) => {
                            let other_value = get(data_path.clone()).await;
                            match (value, other_value) {
                                (None, None) | (Some(Value::Null), Some(Value::Null)) => {
                                    continue;
                                }
                                (Some(Value::Object(a)), Some(Value::Object(b))) => {
                                    if a != &b {
                                        errors.push(validation_rule.clone())
                                    }
                                }
                                (Some(Value::Bool(a)), Some(Value::Bool(b))) => {
                                    if a != &b {
                                        errors.push(validation_rule.clone())
                                    }
                                }
                                (Some(Value::Number(a)), Some(Value::Number(b))) => {
                                    if a != &b {
                                        errors.push(validation_rule.clone())
                                    }
                                }
                                (Some(Value::String(a)), Some(Value::String(b))) => {
                                    if a != &b {
                                        errors.push(validation_rule.clone())
                                    }
                                }
                                (Some(Value::Array(a)), Some(Value::Array(b))) => {
                                    if a != &b {
                                        errors.push(validation_rule.clone())
                                    }
                                }
                                _ => errors.push(validation_rule.clone()),
                            }
                        }
                        ValidationRule::Different(data_path) => {
                            let other_value = get(data_path.clone()).await;
                            match (value, other_value) {
                                (None, None) | (Some(Value::Null), Some(Value::Null)) => {
                                    continue;
                                }
                                (Some(Value::Object(a)), Some(Value::Object(b))) => {
                                    if a == &b {
                                        errors.push(validation_rule.clone())
                                    }
                                }
                                (Some(Value::Bool(a)), Some(Value::Bool(b))) => {
                                    if a == &b {
                                        errors.push(validation_rule.clone())
                                    }
                                }
                                (Some(Value::Number(a)), Some(Value::Number(b))) => {
                                    if a == &b {
                                        errors.push(validation_rule.clone())
                                    }
                                }
                                (Some(Value::String(a)), Some(Value::String(b))) => {
                                    if a == &b {
                                        errors.push(validation_rule.clone())
                                    }
                                }
                                (Some(Value::Array(a)), Some(Value::Array(b))) => {
                                    if a == &b {
                                        errors.push(validation_rule.clone())
                                    }
                                }
                                _ => continue,
                            }
                        }
                        ValidationRule::OneOf(variants) => match value {
                            Some(v) => {
                                if !variants.iter().any(|i| i.0 == *v) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            None => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::NotOneOf(variants) => match value {
                            Some(v) => {
                                if !variants.iter().any(|i| i.0 == *v) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            None => {}
                        },
                        ValidationRule::Min(min) => match value {
                            Some(Value::String(v)) => {
                                if !(v.len() > *min as usize) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::Max(max) => match value {
                            Some(Value::String(v)) => {
                                if !(v.len() < *max as usize) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                continue;
                            }
                        },
                        ValidationRule::StartsWith(start) => match value {
                            Some(Value::String(v)) => {
                                if !v.starts_with(start) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::DoesntStartWith(start) => match value {
                            Some(Value::String(v)) => {
                                if v.starts_with(start) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::EndsWith(end) => match value {
                            Some(Value::String(v)) => {
                                if !v.ends_with(end) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::DoesntEndWith(end) => match value {
                            Some(Value::String(v)) => {
                                if v.ends_with(end) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::Email => match value {
                            Some(Value::String(v)) => {
                                if !is_valid_email(v) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::HexColor => match value {
                            Some(Value::String(v)) => {
                                if !is_valid_hex_color(v) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::Ip(ip) => match value {
                            Some(Value::String(v)) => {
                                if !is_valid_ip(v.as_str(), ip) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::MAC => match value {
                            Some(Value::String(v)) => {
                                if !is_valid_mac_address(v) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::JSON => match value {
                            Some(Value::String(v)) => {
                                if !is_valid_json(v) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::Lowercase => match value {
                            Some(Value::String(v)) => {
                                if !v.chars().all(|c| !c.is_uppercase()) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::Uppercase => match value {
                            Some(Value::String(v)) => {
                                if !v.chars().all(|c| c.is_uppercase()) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::Regex(regex) => {
                            match value {
                                Some(Value::String(v)) => {
                                    let re_opt = Regex::new(regex);
                                    match re_opt {
                                        Ok(re) => {
                                            if !re.is_match(v) {
                                                errors.push(validation_rule.clone());
                                            }
                                        }
                                        Err(_) => {
                                            // TODO: add tracing logs for err
                                            errors.push(validation_rule.clone());
                                        }
                                    }
                                }
                                _ => {
                                    errors.push(validation_rule.clone());
                                }
                            }
                        }
                        ValidationRule::NotRegex(regex) => {
                            match value {
                                Some(Value::String(v)) => {
                                    let re_opt = Regex::new(regex);
                                    match re_opt {
                                        Ok(re) => {
                                            if re.is_match(v) {
                                                errors.push(validation_rule.clone());
                                            }
                                        }
                                        Err(_) => {
                                            // TODO: add tracing logs for err
                                            errors.push(validation_rule.clone());
                                        }
                                    }
                                }
                                _ => {
                                    errors.push(validation_rule.clone());
                                }
                            }
                        }
                        ValidationRule::URL => match value {
                            Some(Value::String(v)) => {
                                if !is_valid_url(v) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::ULID => match value {
                            Some(Value::String(v)) => {
                                if !is_valid_ulid(v) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::UUID(uuid_opt) => match value {
                            Some(Value::String(v)) => {
                                if !is_valid_uuid(v, uuid_opt) {
                                    errors.push(validation_rule.clone());
                                }
                            }
                            _ => {
                                errors.push(validation_rule.clone());
                            }
                        },
                        ValidationRule::Custom(handler) => {
                            match RYADNO_FIELDS_VALIDATION_CLOUSRES
                                .iter()
                                .find(|closure| closure.0 == handler.as_str())
                            {
                                Some(closure) => {
                                    if !(closure.1)(
                                        self.get_data_path(),
                                        form_context.clone(),
                                        runtime_ctx.clone(),
                                        get.clone(),
                                        set.clone(),
                                    )
                                    .await
                                    {
                                        errors.push(validation_rule.clone());
                                    }
                                }
                                None => {}
                            }
                        }
                        _ => {
                            errors.push(validation_rule.clone());
                        }
                    }
                }

                if errors.len() > 0 {
                    return Err(errors);
                }

                Ok(())
            }
            None => Ok(()),
        }
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

    fn get_column_span(&self) -> u8 {
        self.column_span
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
