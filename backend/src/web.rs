use actix_web::{middleware, web, App, HttpServer};
use dotenvy::dotenv;

use kueaplan_backend::api::{configure_app, AppState};
use log::warn;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let dotenv_result = dotenv();
    env_logger::init();
    if dotenv_result.is_err() {
        warn!("Could not read .env file: {}", dotenv_result.unwrap_err());
    }

    let state = AppState::new().unwrap();
    HttpServer::new(move || {
        let api_scope = web::scope("/api/v1").configure(configure_app);
        App::new()
            .service(api_scope)
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::Compress::default())
    })
    .bind(("127.0.0.1", 9000))?
    .run()
    .await
}
