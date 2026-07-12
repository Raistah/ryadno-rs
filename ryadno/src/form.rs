use std::{any::Any, collections::HashMap, fmt::Debug, ops::DerefMut, sync::Arc};

use futures::{FutureExt, future::BoxFuture};
use minijinja::context;
use rkyv::{Archive, Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    fields::{Field, LiveType},
    push_change, render_registry_pusher,
    structs::{
        data_path::DataPath,
        error::Error,
        field_change::{self, FieldChange},
        rkyv::value_wrapper::ValueWrapper,
    },
    value_getter, value_setter,
};

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Form<T: Field + Archive + Debug + Eq + PartialEq> {
    pub schema: Vec<T>,
    pub form_ctx: Arc<FormContext>,
    pub data: Option<ValueWrapper>,
    pub columns: u8,
}

impl<T> Form<T>
where
    T: Field + Archive + Debug + Eq + PartialEq,
{
    pub fn new(schema: Vec<T>, form_ctx: Arc<FormContext>, data: Option<ValueWrapper>) -> Self {
        Self {
            schema,
            form_ctx,
            data,
            columns: 1,
        }
    }

    pub fn columns(mut self, value: u8) -> Self {
        self.columns = value;
        self
    }

    pub async fn to_html<'a, C: Any + Sync + Send>(
        &mut self,
        mjenv: &minijinja::Environment<'_>,
        runtime_ctx: impl Into<Option<Arc<C>>>,
    ) -> Result<String, Error> {
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
                .initial_hydration(
                    self.form_ctx.clone(),
                    ctx.clone(),
                    getter.clone(),
                    setter.clone(),
                )
                .await;
        }

        for field in self.schema.iter() {
            rendered_fields.push_str(
                field
                    .to_html(
                        mjenv,
                        getter(field.get_data_path()).await.as_ref(),
                        self.form_ctx.clone(),
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

        Ok(mjenv.get_template("ryadno/form.jinja")?.render(context! {
            uuid => self.form_ctx.uuid,
            html => rendered_fields,
            columns => self.columns
        })?)
    }

    pub async fn to_html_no_ctx(
        &mut self,
        mjenv: &minijinja::Environment<'_>,
    ) -> Result<String, Error> {
        self.to_html::<()>(mjenv, None).await
    }

    pub async fn handle_update<'a, C: Any + Sync + Send>(
        &mut self,
        update: Update,
        mjenv: &minijinja::Environment<'_>,
        runtime_ctx: impl Into<Option<Arc<C>>>,
    ) -> Vec<FieldChange> {
        let update_arc = Arc::new(update);

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

        let render_registry_clone = render_registry.clone();
        let render_registry_pusher = render_registry_pusher!(render_registry_clone);

        // Updated field should be updated first, so other fields that is depends on it can access its actual value with getter
        for field in self.schema.iter_mut() {
            if update_arc.path.includes(field.get_data_path().as_ref()) {
                let value_amx_clone = value_amx.clone();
                let setter_updated: ValueSetter = value_setter!(value_amx_clone);
                field
                    .process_update(
                        update_arc.clone(),
                        self.form_ctx.clone(),
                        ctx.clone(),
                        getter.clone(),
                        setter_updated.clone(),
                        render_registry_pusher.clone(),
                    )
                    .await;
            }
        }

        // check other fields that depends on updated field
        for field in self.schema.iter_mut() {
            // TODO: in this implementation only if this field is not live then live children will not be proccessed
            // Potential solution is to propagate child live param on parent too. So repeater schema will be parsed, and all the deps will be assigned to repeater itself (on initial_hydration step)
            if update_arc.path != *field.get_data_path()
                && match field.is_live() {
                    LiveType::Static(v) => v.clone(),
                    LiveType::Conditinal(deps) => {
                        let mut res = false;
                        for dep in deps {
                            if dep.includes(&update_arc.path) {
                                res = true;
                                break;
                            }
                        }
                        res
                    }
                }
            {
                field
                    .after_update(
                        update_arc.clone(),
                        self.form_ctx.clone(),
                        ctx.clone(),
                        getter.clone(),
                        setter.clone(),
                        render_registry_pusher.clone(),
                    )
                    .await;
            }
        }

        // render all the changed fields
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
                    self.form_ctx.clone(),
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
                    self.form_ctx.clone(),
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

    pub async fn handle_update_no_ctx(
        &mut self,
        update: Update,
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
    ($value:expr, $registry:expr) => {
        Arc::new(
            move |data_path: Arc<$crate::structs::data_path::DataPath>,
                  value: $crate::serde_json::Value|
                  -> BoxFuture<Option<$crate::structs::data_path::DataPath>> {
                let lock_handle = Arc::clone(&$value);
                let registry_handle = Arc::clone(&$registry);

                async move {
                    let mut lock = lock_handle.lock().await;
                    let data = lock.deref_mut();
                    let result = data_path.set_value(
                        data,
                        value,
                        $crate::structs::data_path::ValueUpdateStrategy::Flex,
                    );

                    let mut registry_lock = registry_handle.lock().await;
                    if !registry_lock.contains(&data_path) {
                        registry_lock.push(data_path);
                    }
                    result
                }
                .boxed()
            },
        )
    };
}

pub type RenderRegistryPusher<'a> = Arc<dyn Fn(Arc<DataPath>) -> BoxFuture<'a, ()> + Send + Sync>;
#[macro_export]
macro_rules! render_registry_pusher {
    ($registry:expr) => {
        Arc::new(
            move |data_path: Arc<$crate::structs::data_path::DataPath>| -> BoxFuture<()> {
                let registry_handle = Arc::clone(&$registry);

                async move {
                    let mut registry_lock = registry_handle.lock().await;
                    if !registry_lock.contains(&data_path) {
                        registry_lock.push(data_path);
                    }
                }
                .boxed()
            },
        )
    };
}

pub type ChangePusher<'a> = &'a mut (dyn FnMut(Arc<DataPath>, field_change::ChangeType) + Send);
#[macro_export]
macro_rules! push_change {
    ($changes:expr) => {
        // Creates a standard synchronous closure capturing a mutable reference
        &mut |data_path: std::sync::Arc<$crate::structs::data_path::DataPath>,
              result: $crate::structs::field_change::ChangeType| {
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
    pub uuid: String,
    pub update_endpoint: String,
    pub headers: HashMap<String, String>,
    pub extra: HashMap<String, String>,
}

impl FormContext {
    pub fn new(
        update_endpoint: String,
        headers: HashMap<String, String>,
        extra: HashMap<String, String>,
    ) -> Self {
        FormContext {
            uuid: Uuid::new_v4().to_string(),
            update_endpoint,
            headers,
            extra,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Update {
    pub uuid: String,
    pub field_uuid: String,
    pub path: DataPath,
    pub update: UpdateType,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum UpdateType {
    Value(Value),
    Action(Value),
}

#[cfg(test)]
mod test {
    use crate::{async_closure, structs::field_dep::FieldDep};
    use linkme::distributed_slice;
    use minijinja::{Environment, path_loader};
    use serde_json::json;

    use crate::fields::{
        BoolValue, FieldTypes,
        text_input::{RYADNO_FIELDS_TEXTFIELD_BOOL_CLOUSRES, TextField, TextFieldHiddenClosure},
    };

    use super::*;

    #[tokio::test]
    async fn test_rkyv_with_generic() {
        let form = Form::new(
            vec![
                TextField::make("first_name".to_string()).into(),
                TextField::make("last_name".to_string()).into(),
            ],
            Arc::new(FormContext::new(
                "".to_string(),
                HashMap::new(),
                HashMap::new(),
            )),
            None,
        );

        let bytes = to_bytes!(&form).unwrap();
        let restored_form = from_bytes!(Form<FieldTypes>, &bytes).unwrap();
        assert_eq!(form, restored_form);
    }

    #[tokio::test]
    async fn test_form_to_html() {
        let mut form: Form<FieldTypes> = Form::new(
            vec![
                TextField::make("first_name".to_string()).into(),
                TextField::make("last_name".to_string()).into(),
            ],
            Arc::new(FormContext::new(
                "".to_string(),
                HashMap::new(),
                HashMap::new(),
            )),
            Some(ValueWrapper(json!({
                "first_name": "hehe",
                "last_name": "hoho"
            }))),
        );

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

    #[distributed_slice(RYADNO_FIELDS_TEXTFIELD_BOOL_CLOUSRES)]
    pub static TEST3_CLOSURE: (&'static str, TextFieldHiddenClosure) = (
        "TEST3_CLOSURE",
        async_closure!((_, _, ctx, _, set) {
            if let Some(any) = ctx
                && let Some(ctx) = any.downcast_ref::<HashMap<String, bool>>()
            {
                set(Arc::new(DataPath::from("test1")), Value::String("test1 value".to_string())).await;
                return ctx.get(&"test2".to_string()).unwrap_or(&false).clone();
            }
            false
        }),
    );

    #[distributed_slice(RYADNO_FIELDS_TEXTFIELD_BOOL_CLOUSRES)]
    pub static TEST4_CLOSURE: (&'static str, TextFieldHiddenClosure) = (
        "TEST4_CLOSURE",
        async_closure!((_, _, _, get, _) {
            if
                Some(Value::String("test3 changed".to_string())) ==
                get(Arc::new(DataPath::from("test3"))).await
            {
                return true;
            }
            false
        }),
    );

    #[tokio::test]
    async fn test_form_full_cycle() {
        let mut ctx: HashMap<String, bool> = HashMap::new();
        ctx.insert("test2".to_string(), true);
        let ctx = Arc::new(ctx);

        let mut form: Form<FieldTypes> = Form::new(
            vec![
                TextField::make("test1".to_string())
                    .hidden(BoolValue::Static(true))
                    .into(),
                // Default value is set when closure not found
                TextField::make("test2".to_string())
                    .hidden(BoolValue::Closure("this_closure_not_exists".into()))
                    .into(),
                TextField::make("test3".to_string())
                    .hidden(BoolValue::Closure(TEST3_CLOSURE.0.into()))
                    .into(),
                TextField::make("test4".to_string())
                    .hidden(BoolValue::Closure(TEST4_CLOSURE.0.into()))
                    .live(LiveType::Conditinal(vec![FieldDep::from("test3")]))
                    .into(),
            ],
            Arc::new(FormContext::new(
                "".to_string(),
                HashMap::new(),
                HashMap::new(),
            )),
            Some(ValueWrapper(json!({
                "test1": "_",
                "test2": "test2 value",
                "test4": "_"
            }))),
        );

        let mut env = Environment::new();
        env.set_loader(move |name| {
            let real_name = name.replace("ryadno/", "");
            path_loader("./src/templates")(real_name.as_str())
        });

        let html = form.to_html(&env, ctx.clone()).await.unwrap();
        assert!(
            !html.contains(format!("field_{}", form.schema.get(0).unwrap().get_uuid()).as_str())
        );
        assert!(
            html.contains(format!("field_{}", form.schema.get(1).unwrap().get_uuid()).as_str())
        );
        assert!(
            !html.contains(format!("field_{}", form.schema.get(2).unwrap().get_uuid()).as_str())
        );
        assert!(
            html.contains(format!("field_{}", form.schema.get(3).unwrap().get_uuid()).as_str())
        );

        // Simulate form update
        let field_to_update = form.schema.get(2).unwrap();
        let field_uuid = field_to_update.get_uuid().to_string();
        let path = field_to_update.get_data_path().as_ref().clone();
        let update = Update {
            uuid: form.form_ctx.uuid.clone(),
            field_uuid: field_uuid.clone(),
            path: path.clone(),
            update: UpdateType::Value(Value::String("test3 changed".to_string())),
        };

        let changes = form.handle_update(update, &env, ctx).await;

        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].data_path,
            *form.schema.get(3).unwrap().get_data_path()
        );
    }
}
