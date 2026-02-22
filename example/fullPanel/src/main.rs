use axum::{
    Form, Router,
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use minijinja::{Environment, path_loader};
use ryadno::{
    PanelBuilder,
    structs::register_page::{RegisterPage, RegisterPageForm},
};

#[tokio::main]
async fn main() {
    let mut env = Environment::new();
    env.set_loader(path_loader("templates"));

    let mut register_page = RegisterPage::new("register-page.jinja".to_string(), async |form: String| {Ok(())}, None);

    let router = Router::new().merge(
        PanelBuilder::new()
            .set_auth_middleware(middleware::from_fn(auth_middleware))
            .set_register_page(register_page)
            .build(env),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

async fn auth_middleware(request: Request, next: Next) -> Response {
    println!("I don't care about security!!");

    let response = next.run(request).await;

    response
}
