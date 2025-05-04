use crate::data_store::auth_token::{AuthToken, Privilege};
use crate::data_store::models::Event;
use crate::web::ui;
use crate::web::ui::error::AppError;
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
    /// If the current page belongs to the context of an event, the authentication info of the user
    /// - if it is known. Used for generating the correct navigation buttons
    pub auth_token: Option<&'a AuthToken>,
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

    pub fn can_manage_entries(&self) -> bool {
        let event_id = if let Some(event) = self.event {
            event.id
        } else {
            return false;
        };
        self.auth_token
            .is_some_and(|t| t.has_privilege(event_id, Privilege::ManageEntries))
    }

    /// Generate the url for the 'add entry' button.
    ///
    /// Requires `event` to be Some.
    pub fn new_entry_form_url(&self) -> Result<String, AppError> {
        let mut url = self.request.url_for(
            "new_entry_form",
            &[self
                .event
                .ok_or(AppError::InternalError(
                    "Cannot generate new_entry_form URL, because `event` is not present".to_owned(),
                ))?
                .id
                .to_string()],
        )?;
        url.set_query(Some(&serde_urlencoded::to_string(
            crate::web::ui::endpoints::edit_entry::NewEntryQueryParams {
                date: self.current_date,
            },
        )?));
        Ok(url.to_string())
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, b| {
        let _ = write!(output, "{:02x}", b);
        output
    })
}
