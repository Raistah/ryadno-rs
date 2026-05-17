use std::{any::Any, collections::HashMap, fmt::Debug, ops::DerefMut, sync::Arc};

use futures::{FutureExt, future::BoxFuture};
use minijinja::context;
use rkyv::{
    Archive, Deserialize, Serialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    fields::{Field, LiveType},
    push_change,
    structs::{
        data_path::DataPath,
        field_change::{FieldChange, FieldChangeResult},
    },
    value_getter, value_setter,
};

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Form<T: Field + Archive + Debug + Eq + PartialEq> {
    pub schema: Vec<T>,
    pub uuid: String,
    pub form_ctx: FormContext,
    pub data: Option<ValueWrapper>,
}

impl<T> Form<T>
where
    T: Field + Archive + Debug + Eq + PartialEq,
{
    async fn to_html<'a, C: Any + Sync + Send>(
        &mut self,
        mjenv: &minijinja::Environment<'_>,
        runtime_ctx: impl Into<Option<Arc<C>>>,
    ) -> Result<String, minijinja::Error> {
        let ctx_opt = runtime_ctx.into();
        let ctx: Option<Arc<dyn Any + Send + Sync>> =
            ctx_opt.map(|arc_c| arc_c as Arc<dyn Any + Send + Sync>);

        let mut rendered_fields = String::new();

        let value_amx = match &mut self.data {
            Some(value) => Arc::new(Mutex::new(std::mem::take(&mut value.0))),
            None => Arc::new(Mutex::new(Value::Null)),
        };

        let value_amx_clone = value_amx.clone();
        let getter = value_getter!(value_amx_clone);

        let value_amx_clone = value_amx.clone();
        let setter = value_setter!(value_amx_clone);

        for field in self.schema.iter_mut() {
            let data_path = Arc::new(DataPath::from(field.get_name()));
            field.set_data_path(data_path.clone());

            field
                .initial_hydration(&self.form_ctx, ctx.clone(), getter.clone(), setter.clone())
                .await;
        }

        for field in self.schema.iter() {
            rendered_fields.push_str(
                field
                    .to_html(
                        mjenv,
                        getter(field.get_data_path()).await.as_ref(),
                        &self.form_ctx,
                    )?
                    .as_str(),
            );
        }

        let mut lock = value_amx.lock().await;
        match &mut self.data {
            Some(value) => {
                value.0 = std::mem::take(&mut *lock);
            }
            None => self.data = Some(ValueWrapper(std::mem::take(&mut *lock))),
        };

        mjenv.get_template("ryadno/form.jinja")?.render(context! {
            html => rendered_fields
        })
    }

    async fn to_html_no_ctx(
        &mut self,
        mjenv: &minijinja::Environment<'_>,
    ) -> Result<String, minijinja::Error> {
        self.to_html::<()>(mjenv, None).await
    }

    async fn handle_update<'a, C: Any + Sync + Send>(
        &mut self,
        update: UpdatePayload,
        mjenv: &minijinja::Environment<'_>,
        runtime_ctx: impl Into<Option<Arc<C>>>,
    ) -> Vec<FieldChange> {
        let ctx_opt = runtime_ctx.into();
        let ctx: Option<Arc<dyn Any + Send + Sync>> =
            ctx_opt.map(|arc_c| arc_c as Arc<dyn Any + Send + Sync>);

        let render_registry: Arc<Mutex<Vec<Arc<DataPath>>>> = Arc::new(Mutex::new(Vec::new()));

        let value_amx = match &mut self.data {
            Some(value) => Arc::new(Mutex::new(std::mem::take(&mut value.0))),
            None => Arc::new(Mutex::new(Value::Null)),
        };

        let value_amx_clone = value_amx.clone();
        let getter: ValueGetter = value_getter!(value_amx_clone);

        let value_amx_clone = value_amx.clone();
        let render_registry_clone = render_registry.clone();
        let setter: ValueSetter = value_setter!(value_amx_clone, render_registry_clone);

        todo!("assign value form update or process action");

        for field in self.schema.iter_mut() {
            let state_path = Arc::new(DataPath::from(field.get_name()));
            if match field.is_live() {
                LiveType::Static(v) => v.clone(),
                LiveType::Conditinal(deps) => {
                    let mut res = false;
                    for dep in deps {
                        if dep.includes(state_path.as_ref()) {
                            res = true;
                            break;
                        }
                    }
                    res
                }
            } {
                field
                    .after_update(&self.form_ctx, ctx.clone(), getter.clone(), setter.clone())
                    .await;
            }
        }

        let mut changes: Vec<FieldChange> = Vec::new();
        let change_push: ChangePusher = push_change!(changes);
        let mut render_registry_lock = render_registry.lock().await;
        let render_registry_ref = render_registry_lock.deref_mut();
        for field in self.schema.iter() {
            let field_path = field.get_data_path();
            if let Some(index) = render_registry_ref
                .iter()
                .position(|path| *path == field_path)
            {
                field.push_change(
                    mjenv,
                    getter(field_path.clone()).await.as_ref(),
                    &self.form_ctx,
                    render_registry_ref,
                    change_push,
                );
                render_registry_ref.swap_remove(index);
            }

            if render_registry_ref
                .iter()
                .any(|path| path.includes(field_path.as_ref()))
            {
                field.push_change(
                    mjenv,
                    getter(field_path).await.as_ref(),
                    &self.form_ctx,
                    render_registry_ref,
                    change_push,
                );
            }
        }

        let mut lock = value_amx.lock().await;
        match &mut self.data {
            Some(value) => {
                value.0 = std::mem::take(&mut *lock);
            }
            None => self.data = Some(ValueWrapper(std::mem::take(&mut *lock))),
        };

        changes
    }

    async fn handle_update_no_ctx(
        &mut self,
        update: UpdatePayload,
        mjenv: &minijinja::Environment<'_>,
    ) -> Vec<FieldChange> {
        self.handle_update::<()>(update, mjenv, None).await
    }
}

pub type ValueGetter<'a> =
    Arc<dyn Fn(Arc<DataPath>) -> BoxFuture<'a, Option<Value>> + Sync + Send + 'a>;
#[macro_export]
macro_rules! value_getter {
    ($value:expr) => {
        Arc::new(
            move |data_path: Arc<$crate::structs::data_path::DataPath>| -> BoxFuture<Option<Value>> {
                let lock_handle = Arc::clone(&$value);

                async move {
                    let lock = lock_handle.lock().await;
                    data_path.find_value(&lock).map(|v| v.clone())
                }
                .boxed()
            },
        )
    };
}

pub type ValueSetter<'a> =
    Arc<dyn Fn(Arc<DataPath>, Value) -> BoxFuture<'a, Option<DataPath>> + Sync + Send + 'a>;
#[macro_export]
macro_rules! value_setter {
    ($value:expr) => {
        Arc::new(
            move |data_path: Arc<$crate::structs::data_path::DataPath>,
                  value: $crate::serde_json::Value|
                  -> BoxFuture<Option<$crate::structs::data_path::DataPath>> {
                let lock_handle = Arc::clone(&$value);

                async move {
                    let mut lock = lock_handle.lock().await;
                    let data = lock.deref_mut();
                    data_path.set_value(
                        data,
                        value,
                        $crate::structs::data_path::ValueUpdateStrategy::Flex,
                    )
                }
                .boxed()
            },
        )
    };
    ($value:expr, $tracker:expr) => {
        Arc::new(
            move |data_path: Arc<$crate::structs::data_path::DataPath>,
                  value: $crate::serde_json::Value|
                  -> BoxFuture<Option<$crate::structs::data_path::DataPath>> {
                let lock_handle = Arc::clone(&$value);
                let tracker_handle = Arc::clone(&$tracker);

                async move {
                    let mut lock = lock_handle.lock().await;
                    let data = lock.deref_mut();
                    let result = data_path.set_value(
                        data,
                        value,
                        $crate::structs::data_path::ValueUpdateStrategy::Flex,
                    );

                    let mut tracker_lock = tracker_handle.lock().await;
                    if !tracker_lock.contains(&data_path) {
                        tracker_lock.push(data_path);
                    }
                    result
                }
                .boxed()
            },
        )
    };
}

pub type ChangePusher<'a> = &'a mut dyn FnMut(Arc<DataPath>, FieldChangeResult);
#[macro_export]
macro_rules! push_change {
    ($changes:expr) => {
        // Creates a standard synchronous closure capturing a mutable reference
        &mut |data_path: std::sync::Arc<$crate::structs::data_path::DataPath>,
              result: $crate::structs::field_change::FieldChangeResult| {
            // Accesses and pushes straight to your stack-allocated vector
            $changes.push($crate::structs::field_change::FieldChange {
                data_path: (*data_path).clone(),
                result: result,
            });
        }
    };
}

#[macro_export]
macro_rules! to_bytes {
    ($from:expr) => {
        $crate::rkyv::to_bytes::<$crate::rkyv::rancor::Error>($from)
    };
}

#[macro_export]
macro_rules! from_bytes {
    ($type:ty, $bytes:expr) => {
        $crate::rkyv::from_bytes::<$type, $crate::rkyv::rancor::Error>($bytes)
    };
}

#[derive(Archive, Serialize, Deserialize, serde::Serialize, Debug, PartialEq, Eq)]
pub struct FormContext {
    pub update_endpoint: String,
    pub headers: HashMap<String, String>,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdatePayload {
    pub uuid: String,
    pub path: String,
    pub value: Value,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ValueWrapper(pub Value);

impl Archive for ValueWrapper {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        let s = self.0.to_string();
        ArchivedString::resolve_from_str(&s, resolver, out);
    }
}

impl<S> Serialize<S> for ValueWrapper
where
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Self::Archived::serialize_from_str::<_>(self.0.to_string().as_str(), serializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<ValueWrapper, D> for ArchivedString {
    fn deserialize(&self, _: &mut D) -> Result<ValueWrapper, D::Error> {
        let string = self.to_string();
        let value: serde_json::Value = serde_json::from_str(string.as_str()).unwrap();
        Ok(ValueWrapper(value))
    }
}

#[cfg(test)]
mod test {
    use crate::async_closure;
    use linkme::distributed_slice;
    use minijinja::{Environment, path_loader};
    use serde_json::json;

    use crate::fields::{
        BoolValue, FieldTypes,
        text_input::{RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES, TextField, TextFieldHiddenClosure},
    };

    use super::*;

    #[tokio::test]
    async fn test_rkyv_with_generic() {
        let form = Form {
            schema: vec![
                TextField::make("first_name".to_string()).into(),
                TextField::make("last_name".to_string()).into(),
            ],
            form_ctx: FormContext {
                update_endpoint: "".to_string(),
                headers: HashMap::new(),
                extra: HashMap::new(),
            },
            uuid: "".to_string(),
            data: None,
        };

        let bytes = to_bytes!(&form).unwrap();
        let restored_form = from_bytes!(Form<FieldTypes>, &bytes).unwrap();
        assert_eq!(form, restored_form);
    }

    #[tokio::test]
    async fn test_form_to_html() {
        let mut form: Form<FieldTypes> = Form {
            schema: vec![
                TextField::make("first_name".to_string()).into(),
                TextField::make("last_name".to_string()).into(),
            ],
            form_ctx: FormContext {
                update_endpoint: "".to_string(),
                headers: HashMap::new(),
                extra: HashMap::new(),
            },
            uuid: "".to_string(),
            data: Some(ValueWrapper(json!({
                "first_name": "hehe",
                "last_name": "hoho"
            }))),
        };

        let mut env = Environment::new();
        env.set_loader(move |name| {
            let real_name = name.replace("ryadno/", "");
            path_loader("./src/templates")(real_name.as_str())
        });

        let html = form.to_html_no_ctx(&env).await.unwrap();
        assert!(html.contains("value: 'hehe'"));
        assert!(html.contains("path: 'first_name'"));
        assert!(html.contains("value: 'hoho'"));
        assert!(html.contains("path: 'last_name'"));
    }

    #[distributed_slice(RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES)]
    pub static GET_TEST2_USING_RUNTIME_CTX_CLOSURE: (&'static str, TextFieldHiddenClosure) = (
        "GET_TEST2_USING_RUNTIME_CTX_CLOSURE",
        async_closure!((field -> path, _, ctx, _, set) {
            if let Some(any) = ctx
                && let Some(ctx) = any.downcast_ref::<HashMap<String, bool>>()
            {
                set(path, Value::String("test3 value".to_string())).await;
                return ctx.get(&"test2".to_string()).unwrap_or(&false).clone();
            }
            false
        }),
    );

    #[distributed_slice(RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES)]
    pub static GET_TEST3_USING_GETTER_CLOSURE: (&'static str, TextFieldHiddenClosure) = (
        "GET_TEST3_USING_GETTER_CLOSURE",
        async_closure!((_, _, _, get, _) {
            if Some(Value::String("test3 value".to_string())) == get(Arc::new(DataPath::from("test3"))).await {
                return true;
            }
            false
        }),
    );

    #[tokio::test]
    async fn test_form_to_html_dynamic() {
        let mut ctx: HashMap<String, bool> = HashMap::new();
        ctx.insert("test2".to_string(), true);
        let ctx = Arc::new(ctx);

        let mut form: Form<FieldTypes> = Form {
            schema: vec![
                TextField::make("test1".to_string())
                    .hidden(BoolValue::Static(true))
                    .into(),
                // Default value is set when closure not found
                TextField::make("test2".to_string())
                    .hidden(BoolValue::Closure("this_closure_not_exists".into()))
                    .into(),
                TextField::make("test3".to_string())
                    .hidden(BoolValue::Closure(
                        GET_TEST2_USING_RUNTIME_CTX_CLOSURE.0.into(),
                    ))
                    .into(),
                TextField::make("test4".to_string())
                    .hidden(BoolValue::Closure(GET_TEST3_USING_GETTER_CLOSURE.0.into()))
                    .into(),
            ],
            form_ctx: FormContext {
                update_endpoint: "".to_string(),
                headers: HashMap::new(),
                extra: HashMap::new(),
            },
            uuid: "".to_string(),
            data: Some(ValueWrapper(json!({
                "test1": "_",
                "test2": "test2 value",
                "test4": "_"
            }))),
        };

        let mut env = Environment::new();
        env.set_loader(move |name| {
            let real_name = name.replace("ryadno/", "");
            path_loader("./src/templates")(real_name.as_str())
        });

        let html = form.to_html(&env, ctx).await.unwrap();
        assert!(!html.contains(r#"<span class="block">Test1</span>"#));
        assert!(html.contains(r#"<span class="block">Test2</span>"#));
        assert!(!html.contains(r#"<span class="block">Test3</span>"#));
        assert!(!html.contains(r#"<span class="block">Test4</span>"#));
    }
}
