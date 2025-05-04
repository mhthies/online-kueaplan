use crate::data_store::models::Event;
use crate::web::ui;
use crate::web::ui::flash::FlashesInterface;
use crate::web::ui::Resources;
use actix_web::error::UrlGenerationError;
use actix_web::HttpRequest;
use std::fmt::Write;

/// Common template data for all ui templates extending the `base.html` template
///
/// This struct must be a part of the template data structure, as the field `base`.
/// The contained data and functions can be used by the individual template's code, as well.
#[derive(Debug)]
pub struct BaseTemplateContext<'a> {
    /// The HTTP request the template is used to respond to. Used for creating ressource urls and
    /// extracting the flash messages
    pub request: &'a HttpRequest,
    /// HTML title
    pub page_title: &'a str,
    /// If the current page belongs to the context of an event, the information about the event.
    /// This is used for rendering the navigation bar.
    pub event: Option<&'a Event>,
    /// If the current page belongs to the context of an event, and it is clearly associated to a
    /// specific day of the event (e.g. main list for date or entry editing)
    pub current_date: Option<chrono::NaiveDate>,
}

impl BaseTemplateContext<'_> {
    pub fn url_for_static(&self, file: &str) -> Result<String, UrlGenerationError> {
        let mut url = self.request.url_for("static_resources", [file])?;
        url.query_pairs_mut().append_pair(
            "hash",
            &Resources::get(file)
                .map(|f| bytes_to_hex(&f.metadata.sha256_hash()))
                .unwrap_or("unknown".to_string()),
        );
        Ok(url.to_string())
    }

    pub fn get_flashes(&self) -> Vec<ui::flash::FlashMessage> {
        self.request.get_and_clear_flashes()
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, b| {
        let _ = write!(output, "{:02x}", b);
        output
    })
}
