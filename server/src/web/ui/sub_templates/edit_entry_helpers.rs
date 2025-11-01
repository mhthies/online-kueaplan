use crate::data_store::{EntryId, EventId};
use crate::web::ui::error::AppError;
use crate::web::ui::util;
use actix_web::HttpRequest;
use askama::Template;

/// Common navigation bar of all entry editing pages
#[derive(Template)]
#[template(path = "sub_templates/edit_entry_navbar.html")]
pub struct EditEntryNavbar<'a> {
    request: &'a HttpRequest,
    event_id: EventId,
    entry_id: &'a EntryId,
    entry_begin_effective_date: chrono::NaiveDate,
    active_link: EditEntryNavbarActiveLink,
}

impl<'a> EditEntryNavbar<'a> {
    pub fn new(
        request: &'a HttpRequest,
        event_id: EventId,
        entry_id: &'a EntryId,
        entry_begin_effective_date: chrono::NaiveDate,
        active_link: EditEntryNavbarActiveLink,
    ) -> Self {
        Self {
            request,
            event_id,
            entry_id,
            entry_begin_effective_date,
            active_link,
        }
    }

    fn clone_entry_form_url(&self) -> Result<String, AppError> {
        let mut url = self
            .request
            .url_for("new_entry_form", &[self.event_id.to_string()])?;
        url.set_query(Some(&serde_urlencoded::to_string(
            crate::web::ui::endpoints::edit_entry::NewEntryQueryParams {
                date: None,
                clone_from: Some(*self.entry_id),
            },
        )?));
        Ok(url.to_string())
    }
}

#[derive(PartialEq)]
pub enum EditEntryNavbarActiveLink {
    EditEntry,
    PreviousDatesOverview,
    DeleteEntry,
}
