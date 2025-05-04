use super::util;
use crate::data_store::{EntryId, EventId};
use actix_web::HttpRequest;
use askama::Template;

/// Common navigation bar of all entry editing pages
#[derive(Template)]
#[template(path = "_edit_entry_navbar.html")]
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
}

#[derive(PartialEq)]
pub enum EditEntryNavbarActiveLink {
    EditEntry,
    PreviousDatesOverview,
    DeleteEntry,
}
