use crate::web::ui::framework::error_page::error_page_middleware;
use crate::web::ui::framework::flash::flash_middleware;
use actix_web::error::UrlGenerationError;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::middleware::from_fn;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder, ResponseError};
use error::AppError;
use framework::flash::FlashesInterface;
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

#[derive(Debug)]
struct BaseTemplateContext<'a> {
    request: &'a HttpRequest,
    page_title: &'a str,
}

impl BaseTemplateContext<'_> {
    fn url_for_static(&self, file: &str) -> Result<String, UrlGenerationError> {
        let mut url = self.request.url_for("static_resources", &[file])?;
        url.query_pairs_mut().append_pair(
            "hash",
            &Resources::get(file)
                .map(|f| bytes_to_hex(&f.metadata.sha256_hash()))
                .unwrap_or("unknown".to_string()),
        );
        Ok(url.to_string())
    }

    fn get_flashes(&self) -> Vec<framework::flash::FlashMessage> {
        self.request.get_and_clear_flashes()
    }
}

async fn not_found_handler() -> Result<&'static str, AppError> {
    Err(AppError::PageNotFound)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
