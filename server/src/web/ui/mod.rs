use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::middleware::from_fn;
use actix_web::{get, web, HttpResponse, Responder};
use error::AppError;
use error_page::error_page_middleware;
use flash::flash_middleware;
use rust_embed::Embed;

mod askama_filters;
pub mod base_template;
mod colors;
mod endpoints;
mod error;
pub mod error_page;
pub mod flash;
mod form_inputs;
mod form_values;
mod time_calculation;
mod util;
pub mod validation;

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
        .service(endpoints::main_list::main_list)
        .service(endpoints::auth::login_form)
        .service(endpoints::auth::login)
        .service(endpoints::edit_entry::edit_entry_form)
        .service(endpoints::edit_entry::edit_entry)
        .service(endpoints::edit_entry::new_entry_form)
        .service(endpoints::edit_entry::new_entry)
        .service(endpoints::previous_dates::previous_dates_overview)
        .service(endpoints::previous_dates::delete_previous_date)
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
