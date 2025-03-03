use crate::data_store::get_store_from_env;
use actix_web::{middleware, web, App, HttpServer};
use std::env;
use std::fmt::Display;
use std::sync::Arc;

mod api;
mod ui;

#[derive(Debug)]
pub enum ApplicationStartupError {
    BindError(std::io::Error),
    ServerError(std::io::Error),
    SetupError(String),
}

impl Display for ApplicationStartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplicationStartupError::BindError(e) => {
                write!(f, "Error binding listening port: {}", e)
            }
            ApplicationStartupError::ServerError(e) => write!(f, "Error running web server: {}", e),
            ApplicationStartupError::SetupError(e) => write!(
                f,
                "Error in environment settings for online-kueaplan: {}",
                e
            ),
        }
    }
}

pub fn serve() -> Result<(), ApplicationStartupError> {
    let state = AppState::new().map_err(ApplicationStartupError::SetupError)?;
    actix_web::rt::System::new()
        .block_on(
            HttpServer::new(move || {
                App::new()
                    .configure(api::configure_app)
                    .configure(ui::configure_app)
                    .app_data(web::Data::new(state.clone()))
                    .wrap(middleware::Compress::default())
            })
            .bind(("127.0.0.1", 9000))
            .map_err(ApplicationStartupError::BindError)?
            .run(),
        )
        .map_err(ApplicationStartupError::ServerError)
}

#[derive(Clone)]
pub struct AppState {
    store: Arc<dyn crate::data_store::KuaPlanStore>,
    secret: String,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            store: Arc::new(get_store_from_env()?),
            secret: env::var("SECRET").map_err(|_| "SECRET must be set")?,
        })
    }
}
