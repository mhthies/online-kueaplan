use crate::web::ui::framework::error_page::error_page_middleware;
use crate::web::ui::framework::flash::flash_middleware;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::middleware::from_fn;
use actix_web::{get, web, HttpResponse, Responder};
use error::AppError;
use rust_embed::Embed;

mod auth;
mod edit_entry;
mod error;
mod framework;
mod main_list;
mod util;

#[allow(clippy::identity_op)] // We want to explicitly state that it's "1" year
const SESSION_COOKIE_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(1 * 86400 * 365);

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(
        get_ui_service()
            .wrap(from_fn(flash_middleware))
            .wrap(from_fn(error_page_middleware)),
    );
}

fn get_ui_service() -> actix_web::Scope {
    web::scope("/ui")
        .service(static_resources)
        .service(main_list::main_list)
        .service(auth::login_form)
        .service(auth::login)
        .service(edit_entry::edit_entry_form)
        .service(edit_entry::edit_entry)
        .default_service(web::to(not_found_handler))
}

#[derive(Embed)]
#[folder = "static/"]
struct Resources;

impl Resources {
    fn handle_embedded_file(path: &str) -> HttpResponse {
        match Self::get(path) {
            Some(content) => HttpResponse::Ok()
                .content_type(mime_guess::from_path(path).first_or_octet_stream().as_ref())
                .append_header(CacheControl(vec![CacheDirective::MaxAge(86400 * 365)]))
                .body(content.data.into_owned()),
            None => {
                HttpResponse::NotFound().body(format!("Static resource file '{}' not found", path))
            }
        }
    }
}

#[get("/static/{_:.*}")]
async fn static_resources(path: web::Path<String>) -> impl Responder {
    Resources::handle_embedded_file(path.as_str())
}

async fn not_found_handler() -> Result<&'static str, AppError> {
    Err(AppError::PageNotFound)
}
