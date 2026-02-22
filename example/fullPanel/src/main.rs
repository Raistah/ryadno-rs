use axum::{Router, extract::Request, middleware::Next, response::Response};
use minijinja::{Environment, path_loader};
use ryadno::{
    PanelBuilder,
    structs::register_page::{BaseRegistrationPage, RegisterPageForm},
};

#[tokio::main]
async fn main() {
    let mut env = Environment::new();
    env.set_loader(path_loader("templates"));
    let registration_page = BaseRegistrationPage::new(
        async |form: RegisterPageForm| {
            println!("{:?}", form);
            Ok(())
        },
        "register-page.jinja".to_string(),
    );

    let router = Router::new().merge(
        PanelBuilder::new()
            .with_registration(registration_page)
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
