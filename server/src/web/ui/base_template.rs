use crate::data_store::auth_token::{AuthToken, Privilege};
use crate::data_store::models::{Event, ExtendedEvent};
use crate::web::ui;
use crate::web::ui::error::AppError;
use crate::web::ui::flash::FlashesInterface;
use crate::web::ui::Resources;
use actix_web::error::UrlGenerationError;
use actix_web::HttpRequest;
use std::fmt::Write;

#[derive(Debug)]
pub enum AnyEventData<'a> {
    ExtendedEvent(&'a ExtendedEvent),
    BasicEvent(&'a Event),
    None,
}

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
    /// If the current page belongs to the context of an event, the information about the event, to
    /// the level of detail that is available (either an [[ExtendedEvent]] or at least an [[Event]].
    /// This is used for rendering the navigation bar.
    pub event: AnyEventData<'a>,
    /// If the current page belongs to the context of an event, and it is clearly associated to a
    /// specific day of the event (e.g. main list for date or entry editing)
    pub current_date: Option<chrono::NaiveDate>,
    /// If the current page belongs to the context of an event, the authentication info of the user
    /// - if it is known. Used for generating the correct navigation buttons
    pub auth_token: Option<&'a AuthToken>,
    pub active_main_nav_button: Option<MainNavButton>,
}

impl<'a> BaseTemplateContext<'a> {
    /// Get basic data about the event to which the current page belongs, if available.
    pub fn get_basic_event(&self) -> Option<&'a Event> {
        match self.event {
            AnyEventData::ExtendedEvent(e) => Some(&e.basic_data),
            AnyEventData::BasicEvent(e) => Some(e),
            AnyEventData::None => None,
        }
    }

    /// Get extended data about the event to which the current page belongs, if available.
    /// If only basic event data is available (e.g. because user is not authorized for ShowKueaPlan
    /// privilege), this function return None.
    pub fn get_extended_event(&self) -> Option<&'a ExtendedEvent> {
        match self.event {
            AnyEventData::ExtendedEvent(e) => Some(e),
            AnyEventData::BasicEvent(_) => None,
            AnyEventData::None => None,
        }
    }

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

    pub fn has_privilege(&self, privilege: Privilege) -> bool {
        let event_id = if let Some(event) = self.get_basic_event() {
            event.id
        } else {
            return false;
        };
        self.auth_token
            .is_some_and(|t| t.has_privilege(event_id, privilege))
    }

    /// Generate the url for the 'add entry' button.
    ///
    /// Requires `event` to be Some.
    pub fn new_entry_form_url(&self) -> Result<String, AppError> {
        let mut url = self.request.url_for(
            "new_entry_form",
            &[self
                .get_basic_event()
                .ok_or(AppError::InternalError(
                    "Cannot generate new_entry_form URL, because `event` is not present".to_owned(),
                ))?
                .id
                .to_string()],
        )?;
        url.set_query(Some(&serde_urlencoded::to_string(
            crate::web::ui::endpoints::edit_entry::NewEntryQueryParams {
                date: self.current_date,
                clone_from: None,
            },
        )?));
        Ok(url.to_string())
    }

    /// Get the URL for the given `endpoint_name`, assuming that this endpoint only requires a
    /// single URL placeholder with the current event id.
    pub fn url_for_event_endpoint(&self, endpoint_name: &str) -> Result<String, AppError> {
        Ok(self
            .request
            .url_for(
                endpoint_name,
                &[self
                    .get_basic_event()
                    .ok_or(AppError::InternalError(format!(
                        "Cannot generate URL for {}, because `event` is not present",
                        endpoint_name
                    )))?
                    .id
                    .to_string()],
            )?
            .to_string())
    }

    /// Get the current effective date, if all required information is present to determine it
    pub fn get_current_date_opt(&self) -> Option<chrono::NaiveDate> {
        self.get_extended_event()
            .map(|e| crate::web::time_calculation::current_effective_date(&e.clock_info))
    }
}

#[derive(Debug, PartialEq)]
pub enum MainNavButton {
    ByDate,
    ByCategory,
    ByRoom,
    AddEntry,
    Configuration,
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, b| {
        let _ = write!(output, "{:02x}", b);
        output
    })
}

/// Common template data for all ui templates in the config area, extending the `base_config.html`
/// template.
///
/// This struct must be a part of the template data structure, as the field `base_config`, in
/// addition to the `base` field for the `base.html` template.
#[derive(Debug)]
pub struct BaseConfigTemplateContext {
    pub active_nav_button: ConfigNavButton,
}

#[derive(Debug, PartialEq)]
pub enum ConfigNavButton {
    Overview,
    EventConfig,
    Categories,
    Rooms,
    Passphrases,
    Announcements,
}
