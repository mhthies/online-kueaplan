use crate::web::ui::framework::flash::FlashesInterface;
use crate::web::ui::{framework, Resources};
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

    pub fn get_flashes(&self) -> Vec<framework::flash::FlashMessage> {
        self.request.get_and_clear_flashes()
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, b| {
        let _ = write!(output, "{:02x}", b);
        output
    })
}
