use std::{collections::HashMap, convert::Infallible, error::Error, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures_util::Stream;
use minijinja::{Environment, context, path_loader};
use ryadno::{
    async_closure,
    fields::{
        BoolValue, FieldTypes, LiveType, text_input::{RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES, TextField, TextFieldHiddenClosure}
    },
    form::{Form, FormContext, Update},
    from_bytes,
    linkme::distributed_slice,
    rkyv::util::AlignedVec,
    structs::{data_path::DataPath, field_dep::FieldDep},
    to_bytes,
};
use serde_json::Value;
use tokio::{
    sync::{Mutex, OnceCell},
    time::sleep,
};

struct AppState {
    mjenv: Environment<'static>,
}

static FORMS: OnceCell<Mutex<HashMap<String, (AlignedVec, i64)>>> = OnceCell::const_new();

async fn get_froms() -> &'static Mutex<HashMap<String, (AlignedVec, i64)>> {
    FORMS
        .get_or_init(|| async { Mutex::new(HashMap::new()) })
        .await
}

async fn get_form_by_uuid(uuid: &String) -> Option<Form<FieldTypes>> {
    let forms = get_froms().await;
    let lock = forms.lock().await;
    lock.get(uuid)
        .map(|v| from_bytes!(Form<FieldTypes>, &v.0).unwrap())
}

async fn store_form(form: Form<FieldTypes>) {
    let now = chrono::offset::Utc::now().timestamp();
    let forms = get_froms().await;
    let mut lock = forms.lock().await;
    let bytes = to_bytes!(&form).unwrap();
    lock.insert(form.form_ctx.uuid.clone(), (bytes, now));
}

async fn forms_janitor() {
    let ttl = chrono::offset::Utc::now().timestamp() - 900;
    let forms = get_froms().await;
    let mut lock = forms.lock().await;
    lock.retain(|_, (_, timestamp)| *timestamp > ttl);
}

#[distributed_slice(RYADNO_FIELDS_TEXTFIELD_HIDDEN_CLOUSRES)]
#[linkme(crate = ryadno::linkme)]
pub static HIDE_IF_FIRST_NAME_IS_EMPTY: (&'static str, TextFieldHiddenClosure) = (
    "HIDE_IF_FIRST_NAME_IS_EMPTY",
    async_closure!((_, _, _, get, _) {
        match get(Arc::new(DataPath::from("first_name"))).await {
            Some(Value::String(v)) => {
                v.len() == 0
            },
            _ => true
        }
    }),
);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut mjenv = Environment::new();
    mjenv.set_loader(path_loader("./templates"));

    let state = AppState { mjenv: mjenv };

    tokio::spawn(async {
        loop {
            forms_janitor().await;
            sleep(Duration::from_secs(30)).await;
        }
    });

    let app = Router::new()
        .route("/", get(initial_form_render).post(handle_form_update))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn initial_form_render(State(state): State<Arc<AppState>>) -> axum::response::Html<String> {
    let mut form: Form<FieldTypes> = Form::new(
        vec![
            TextField::make("first_name".into()).into(),
            TextField::make("last_name".into())
                .hidden(BoolValue::Closure(HIDE_IF_FIRST_NAME_IS_EMPTY.0.into()))
                .live(LiveType::Conditinal(vec![FieldDep::from("first_name")]))
                .into(),
        ],
        Arc::new(FormContext::new(
            "/".to_string(),
            HashMap::new(),
            HashMap::new(),
        )),
        None,
    );

    let html = form.to_html_no_ctx(&state.mjenv).await.unwrap();
    store_form(form).await;

    axum::response::Html(
        state
            .mjenv
            .get_template("base.jinja")
            .unwrap()
            .render(context! {
                html => html
            })
            .unwrap(),
    )
}

async fn handle_form_update(
    State(state): State<Arc<AppState>>,
    Json(update): Json<Update>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let mut form = match get_form_by_uuid(&update.uuid).await {
        Some(v) => v,
        None => {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let changes = form.handle_update_no_ctx(update, &state.mjenv).await;
    let event_stream = async_stream::stream! {
        for change in changes {
            let event = change.to_datastar_event(&form.form_ctx.uuid);
            yield Ok(event);
        }
        store_form(form).await;
    };

    Ok(Sse::new(event_stream).keep_alive(KeepAlive::default()))
}
