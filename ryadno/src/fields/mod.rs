// Create sync and async version for methods to use with respective form type
pub mod select;
pub mod text_input;

use std::{any::Any, fmt::Debug, sync::Arc};

use async_trait;
use minijinja::Environment;
use rkyv::Archive;
use serde_json::Value;

use crate::{
    fields::text_input::TextField,
    form::{ChangePusher, FormContext, RenderRegistryPusher, Update, ValueGetter, ValueSetter},
    structs::{data_path::DataPath, error::Error, field_dep::FieldDep},
};

#[async_trait::async_trait]
pub trait Field: Archive + Debug + Eq + PartialEq {
    fn get_data_path(&self) -> Arc<DataPath>;
    fn set_data_path(&mut self, data_path: Arc<DataPath>);

    /// Lifecycle method.
    /// Form calls this method before generate html for the first time
    async fn initial_hydration<'a>(
        &mut self,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
    );

    /// Lifecycle method.
    /// Form calls this method if current field or its child is updated
    async fn process_update<'a>(
        &mut self,
        update: Arc<Update>,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
        push: RenderRegistryPusher<'a>,
    );

    /// Lifecycle method.
    /// Form calls this method after any change of other field if this field live property set to **true**
    async fn after_update<'a>(
        &mut self,
        update: Arc<Update>,
        form_context: Arc<FormContext>,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
        push: RenderRegistryPusher<'a>,
    );

    fn to_html(
        &self,
        mjenv: &Environment<'_>,
        value: Option<&Value>,
        form_context: Arc<FormContext>,
    ) -> Result<String, Error>;

    fn push_change(
        &self,
        mjenv: &minijinja::Environment<'_>,
        value: Option<&serde_json::Value>,
        form_context: Arc<FormContext>,
        render_registry: &mut Vec<Arc<DataPath>>,
        push: ChangePusher,
    );

    fn validate(&self, value: Value) -> Result<(), Vec<(String, String)>>;
    fn get_name(&self) -> &str;
    fn get_uuid(&self) -> &str;
    fn is_live(&self) -> &LiveType;
}

pub fn prepare_value_for_datastar(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Number(v) => format!("{v}"),
        Value::String(v) => format!("'{v}'"),
        Value::Bool(v) => format!("{v}"),
        Value::Array(v) => format!("{}", serde_json::to_string(&v).unwrap()),
        Value::Object(v) => format!("{}", serde_json::to_string(&v).unwrap()),
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

        #[async_trait::async_trait]
        impl Field for $enum_name {
            fn get_data_path(&self) -> Arc<DataPath> {
                match self {
	                $(Self::$variant(v) => v.get_data_path()),*
	            }
            }

            fn set_data_path(&mut self, data_path: Arc<DataPath>) {
                match self {
	                $(Self::$variant(v) => v.set_data_path(data_path)),*
	            }
            }

	        async fn initial_hydration<'a>(
	            &mut self,
	            form_context: Arc<FormContext>,
	            runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
				get: $crate::form::ValueGetter<'a>,
                set: $crate::form::ValueSetter<'a>,
	        ) {
	            match self {
	                $(Self::$variant(v) => v.initial_hydration(form_context, runtime_ctx, get, set)),*
	            }.await
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
                match self {
                    $(Self::$variant(v) => v.process_update(update, form_context, runtime_ctx, get, set, push)),*
                }.await
            }

            async fn after_update<'a>(
                &mut self,
                update: Arc<Update>,
                form_context: Arc<FormContext>,
                runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
                get: $crate::form::ValueGetter<'a>,
                set: $crate::form::ValueSetter<'a>,
                push: $crate::form::RenderRegistryPusher<'a>,
            ) {
                match self {
                    $(Self::$variant(v) => v.after_update(update, form_context, runtime_ctx, get, set, push)),*
                }.await
            }

            fn to_html(
                &self,
                mjenv: &$crate::minijinja::Environment<'_>,
                value: Option<&$crate::serde_json::Value>,
                form_context: Arc<FormContext>,
            ) -> Result<String, $crate::structs::error::Error> {
                match self {
                    $(Self::$variant(v) => v.to_html(mjenv, value, form_context)),*
                }
            }

            fn push_change(
                &self,
                mjenv: &minijinja::Environment<'_>,
                value: Option<&serde_json::Value>,
                form_context: Arc<FormContext>,
                render_registry: &mut Vec<Arc<DataPath>>,
                push: ChangePusher,
            ) {
                match self {
                    $(Self::$variant(v) => v.push_change(mjenv, value, form_context, render_registry, push)),*
                }
            }

            fn validate(&self, value: $crate::serde_json::Value) -> Result<(), Vec<(String, String)>> {
                match self {
                    $(Self::$variant(v) => v.validate(value)),*
                }
            }

            fn is_live(&self) -> &LiveType {
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

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub enum LiveType {
    Static(bool),
    Conditinal(Vec<FieldDep>),
}

#[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
pub enum BoolValue {
    Static(bool),
    Closure(String),
}

register_field_type_enum! {
    FieldTypes {
        Text(TextField),
    }
}

#[macro_export]
macro_rules! async_closure {
    (($($arg:pat),*) $body:block) => {
        |$($arg),*| { Box::pin(async move $body) }
    };
}
