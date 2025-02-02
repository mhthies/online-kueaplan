use actix_web::{middleware, web, App, HttpServer};
use dotenvy::dotenv;

use kueaplan_backend::api::{configure_app, AppState};
use log::warn;

fn main() -> std::io::Result<()> {
    let dotenv_result = dotenv();
    env_logger::init();
    if dotenv_result.is_err() {
        warn!("Could not read .env file: {}", dotenv_result.unwrap_err());
    }

    let state = AppState::new().unwrap();
    actix_web::rt::System::new().block_on(
        HttpServer::new(move || {
            App::new()
                .configure(configure_app)
                .app_data(web::Data::new(state.clone()))
                .wrap(middleware::Compress::default())
        })
        .bind(("127.0.0.1", 9000))?
        .run(),
    )
}
