use crate::ServerState;
use axum::Router;

mod v1;

pub fn auth_routes() -> Router<ServerState> {
	Router::new().nest("/v1", v1::routes())
}
