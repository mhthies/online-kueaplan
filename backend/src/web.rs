
use actix_web::{HttpServer, App, middleware, web};

use kueaplan_backend::api::{AppState, configure_app};


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = AppState::new();
    HttpServer::new(move || {
        App::new()
            .configure(configure_app)
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::Compress::default())
        })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
