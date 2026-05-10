// Create sync and async version for methods to use with respective form type
pub mod select;
pub mod text_input;

use std::{any::Any, fmt::Debug, sync::Arc};

use minijinja::Environment;
use rkyv::Archive;
use serde_json::Value;

use crate::{
    fields::text_input::TextField,
    form::{FormContext, ValueGetter, ValueSetter},
    structs::data_path::DataPath,
};

pub trait Field: Archive + Debug + Eq + PartialEq {
    /// Lifecycle method.
    /// Form calls this method before generate html for the first time
    async fn initial_hydration<'a>(
        &mut self,
        data_path: Arc<DataPath>,
        form_context: &FormContext,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
    );

    /// Lifecycle method.
    /// Form calls this method after any change if this field live property set to **true**
    async fn after_update<'a>(
        &mut self,
        data_path: Arc<DataPath>,
        form_context: &FormContext,
        runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
        get: ValueGetter<'a>,
        set: ValueSetter<'a>,
    );

    fn to_html(
        &self,
        mjenv: &Environment<'_>,
        state_path: &DataPath,
        value: Option<&Value>,
        form_context: &FormContext,
    ) -> Result<String, minijinja::Error>;
    fn validate(&self, value: Value) -> Result<(), Vec<(String, String)>>;
    fn get_name(&self) -> &str;
    fn get_uuid(&self) -> &str;
    fn is_live(&self) -> bool;
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

        impl Field for $enum_name {

	        async fn initial_hydration<'a>(
	            &mut self,
	            data_path: Arc<DataPath>,
	            form_context: &FormContext,
	            runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
				get: $crate::form::ValueGetter<'a>,
                set: $crate::form::ValueSetter<'a>,
	        ) {
	            match self {
	                $(Self::$variant(v) => v.initial_hydration(data_path, form_context, runtime_ctx, get, set)),*
	            }.await
	        }

            async fn after_update<'a>(
                &mut self,
                data_path: Arc<DataPath>,
                form_context: &FormContext,
                runtime_ctx: Option<Arc<dyn Any + Sync + Send>>,
                get: $crate::form::ValueGetter<'a>,
                set: $crate::form::ValueSetter<'a>,
            ) {
                match self {
                    $(Self::$variant(v) => v.after_update(data_path, form_context, runtime_ctx, get, set)),*
                }.await
            }

            fn to_html(
                &self,
                mjenv: &$crate::minijinja::Environment<'_>,
                state_path: &DataPath,
                value: Option<&$crate::serde_json::Value>,
                form_context: &FormContext,
            ) -> Result<String, $crate::minijinja::Error> {
                match self {
                    $(Self::$variant(v) => v.to_html(mjenv, state_path, value, form_context)),*
                }
            }

            fn validate(&self, value: $crate::serde_json::Value) -> Result<(), Vec<(String, String)>> {
                match self {
                    $(Self::$variant(v) => v.validate(value)),*
                }
            }

            fn is_live(&self) -> bool {
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
