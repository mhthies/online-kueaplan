use crate::cli_error::CliError;
use crate::data_store::get_store_from_env;
use crate::setup::{
    get_admin_email_from_env, get_admin_name_from_env, get_listen_address_from_env,
    get_listen_port_from_env, get_secret_from_env,
};
use crate::web::http_error_logging::error_logging_middleware;
use actix_web::error::UrlGenerationError;
use actix_web::web::Redirect;
use actix_web::{get, middleware, web, App, HttpRequest, HttpServer, Responder};
use std::sync::Arc;

mod api;
mod http_error_logging;
mod ical;
mod ui;
mod time_calculation;

pub fn serve() -> Result<(), CliError> {
    let state = AppState::new()?;
    actix_web::rt::System::new()
        .block_on(
            HttpServer::new(move || {
                App::new()
                    .configure(api::configure_app)
                    .configure(ui::configure_app)
                    .service(index)
                    .service(ical::ical)
                    .app_data(web::Data::new(state.clone()))
                    .wrap(actix_web::middleware::from_fn(error_logging_middleware))
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
    admin: AdminInfo,
}

impl AppState {
    pub fn new() -> Result<Self, CliError> {
        Ok(Self {
            store: Arc::new(get_store_from_env()?),
            secret: get_secret_from_env()?,
            admin: AdminInfo {
                name: get_admin_name_from_env()?,
                email: get_admin_email_from_env()?,
            },
        })
    }
}

#[derive(Clone)]
struct AdminInfo {
    name: String,
    email: String,
}

#[get("/")]
async fn index(request: HttpRequest) -> Result<impl Responder, UrlGenerationError> {
    Ok(Redirect::to(
        request.url_for::<_, &&str>("events_list", &[])?.to_string(),
    ))
}
