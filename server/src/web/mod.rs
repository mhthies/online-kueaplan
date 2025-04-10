use crate::cli_error::CliError;
use crate::data_store::get_store_from_env;
use crate::setup::{get_listen_address_from_env, get_listen_port_from_env, get_secret_from_env};
use actix_web::{middleware, web, App, HttpServer};
use std::sync::Arc;

mod api;
mod ui;

pub fn serve() -> Result<(), CliError> {
    let state = AppState::new()?;
    actix_web::rt::System::new()
        .block_on(
            HttpServer::new(move || {
                App::new()
                    .configure(api::configure_app)
                    .configure(ui::configure_app)
                    .app_data(web::Data::new(state.clone()))
                    .wrap(middleware::Compress::default())
            })
            .bind((get_listen_address_from_env()?, get_listen_port_from_env()?))
            .map_err(CliError::BindError)?
            .run(),
        )
        .map_err(CliError::ServerError)
}

#[derive(Clone)]
pub struct AppState {
    store: Arc<dyn crate::data_store::KuaPlanStore>,
    secret: String,
}

impl AppState {
    pub fn new() -> Result<Self, CliError> {
        Ok(Self {
            store: Arc::new(get_store_from_env()?),
            secret: get_secret_from_env()?,
        })
    }
}
