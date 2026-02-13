use axum::routing::MethodRouter;

#[derive(Clone)]
pub struct RegisterPage {
	pub template: String,
	pub handler: MethodRouter,
	pub path: Option<String>,
}
