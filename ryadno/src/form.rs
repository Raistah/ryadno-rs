use std::{collections::HashMap, fmt::Debug};

use minijinja::context;
use rkyv::{
    Archive, Deserialize, Serialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
};
use serde_json::Value;

use crate::{fields::Field, structs::data_path::DataPath};

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
    fn to_html(&mut self, mjenv: &minijinja::Environment<'_>) -> Result<String, minijinja::Error> {
        let mut rendered_fields = String::new();

        match &self.data {
            Some(value) => {
                for field in self.schema.iter_mut() {
                    let state_path = DataPath::from(field.get_name());
                    let value = state_path.find_value(&value.0);

                   	field.initial_hydration(value, &self.form_ctx, None);
                    println!("{:?}", &field);
                    rendered_fields.push_str(
                        field
                            .to_html(
                                mjenv,
                                state_path.clone(),
                                value,
                                &self.form_ctx,
                            )?
                            .as_str(),
                    );
                }
            }
            None => {
                for field in self.schema.iter_mut() {
                    field.initial_hydration(None, &self.form_ctx, None);
                    rendered_fields.push_str(
                        field
                            .to_html(
                                mjenv,
                                DataPath::from(field.get_name()),
                                None,
                                &self.form_ctx,
                            )?
                            .as_str(),
                    );
                }
            }
        }

        mjenv.get_template("ryadno/form.jinja")?.render(context! {
            html => rendered_fields
        })
    }
}

macro_rules! to_bytes {
    ($from:expr) => {
        $crate::rkyv::to_bytes::<$crate::rkyv::rancor::Error>($from)
    };
}

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
    use linkme::distributed_slice;
    use minijinja::{Environment, path_loader};
    use serde_json::json;

    use crate::fields::{
        BoolValue, FieldTypes,
        text_input::{RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES, TextField, TextFieldHiddenClosure},
    };

    use super::*;

    #[test]
    fn test_rkyv_with_generic() {
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

    #[test]
    fn test_form_to_html() {
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

        let html = form.to_html(&env).unwrap();
        assert!(html.contains("value: 'hehe'"));
        assert!(html.contains("path: 'first_name'"));
        assert!(html.contains("value: 'hoho'"));
        assert!(html.contains("path: 'last_name'"));
    }

    #[distributed_slice(RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES)]
    pub static TEST_CLOSURE: (
        &'static str,
        TextFieldHiddenClosure,
    ) = (
        "hidden_closure",
        |_, _, _, _| {
        	true
        },
    );

    #[test]
    fn test_form_to_html_dynamic() {
        let mut form: Form<FieldTypes> = Form {
            schema: vec![
                TextField::make("test1".to_string())
                    .hidden(BoolValue::Static(true))
                    .into(),
                TextField::make("test2".to_string())
                    .hidden(BoolValue::Closure("this_closure_not_exists".into()))
                    .into(),
                TextField::make("test3".to_string())
                    .hidden(BoolValue::Closure("hidden_closure".into()))
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
                "test2": "_",
                "test3": "test"
            }))),
        };

        let mut env = Environment::new();
        env.set_loader(move |name| {
            let real_name = name.replace("ryadno/", "");
            path_loader("./src/templates")(real_name.as_str())
        });

        let html = form.to_html(&env).unwrap();
        assert!(!html.contains(r#"<span class="block">Test1</span>"#));
        assert!(html.contains(r#"<span class="block">Test2</span>"#));
        assert!(!html.contains(r#"<span class="block">Test3</span>"#));
    }
}
