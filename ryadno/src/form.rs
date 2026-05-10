// TODO: fist of all rework everything async way, then create sync version if required
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
    value_getter,
    value_setter,
    fields::Field,
    structs::data_path::{DataPath, ValueUpdateStrategy},
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
        match &mut self.data {
            Some(value) => {
                let value_amx = Arc::new(Mutex::new(std::mem::take(&mut value.0)));
                let value_amx_clone = value_amx.clone();
                let getter = value_getter!(&value_amx_clone);

                let value_amx_clone = value_amx.clone();
                let setter = value_setter!(&value_amx_clone);

                for field in self.schema.iter_mut() {
                    let state_path = Arc::new(DataPath::from(field.get_name()));
                    field
                        .initial_hydration(
                            state_path.clone(),
                            &self.form_ctx,
                            ctx.clone(),
                            getter.clone(),
                            setter.clone(),
                        )
                        .await;

                    let field_value = {
                        let lock = value_amx.lock().await;
                        state_path.find_value(&lock).map(|v| v.clone())
                    };
                    rendered_fields.push_str(
                        field
                            .to_html(
                                mjenv,
                                state_path.as_ref(),
                                field_value.as_ref(),
                                &self.form_ctx,
                            )?
                            .as_str(),
                    );
                }

                let mut lock = value_amx.lock().await;
                value.0 = std::mem::take(&mut *lock);
            }
            None => {
                let value_amx = Arc::new(Mutex::new(Value::Null));
                let value_amx_clone = value_amx.clone();
                let getter = value_getter!(&value_amx_clone);

                let value_amx_clone = value_amx.clone();
                let setter = value_setter!(&value_amx_clone);

                for field in self.schema.iter_mut() {
                    let state_path = Arc::new(DataPath::from(field.get_name()));
                    field
                        .initial_hydration(
                            state_path.clone(),
                            &self.form_ctx,
                            ctx.clone(),
                            getter.clone(),
                            setter.clone(),
                        )
                        .await;

                    rendered_fields.push_str(
                        field
                            .to_html(
                                mjenv,
                                &DataPath::from(field.get_name()),
                                None,
                                &self.form_ctx,
                            )?
                            .as_str(),
                    );
                }

                let mut lock = value_amx.lock().await;
                self.data = Some(ValueWrapper(std::mem::take(&mut *lock)));
            }
        }

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
}

pub type ValueGetter<'a> = Arc<dyn Fn(Arc<DataPath>) -> BoxFuture<'a, Option<Value>> + Sync + Send + 'a>;
#[macro_export]
macro_rules! value_getter {
    ($value:expr) => {
        Arc::new(move |data_path: Arc<DataPath>| -> BoxFuture<Option<Value>> {
            let lock_handle = Arc::clone(&$value);

            async move {
                let lock = lock_handle.lock().await;
                data_path.find_value(&lock).map(|v| v.clone())
            }
            .boxed()
        })
    };
}

pub type ValueSetter<'a> = Arc<dyn Fn(Arc<DataPath>, Value) -> BoxFuture<'a, Option<DataPath>> + Sync + Send + 'a>;
#[macro_export]
macro_rules! value_setter {
    ($value:expr) => {
        Arc::new(
            move |data_path: Arc<DataPath>, value: Value| -> BoxFuture<Option<DataPath>> {
                let lock_handle = Arc::clone(&$value);

                async move {
                    let mut lock = lock_handle.lock().await;
                    let data = lock.deref_mut();
                    data_path.set_value(data, value, ValueUpdateStrategy::Flex)
                }
                .boxed()
            },
        )
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
        async_closure!((_, data_path, _, ctx, _, set) {
            if let Some(any) = ctx
                && let Some(ctx) = any.downcast_ref::<HashMap<String, bool>>()
            {
                set(data_path, Value::String("test3 value".to_string())).await;
                return ctx.get(&"test2".to_string()).unwrap_or(&false).clone();
            }
            false
        }),
    );

    #[distributed_slice(RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES)]
    pub static GET_TEST3_USING_GETTER_CLOSURE: (&'static str, TextFieldHiddenClosure) = (
        "GET_TEST3_USING_GETTER_CLOSURE",
        async_closure!((_, _, _, _, get, _) {
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
