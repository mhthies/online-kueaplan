use crate::data_store::models::FullEntry;
use crate::web::ui::sub_templates::main_list_row::RoomByIdWithOrder;
use askama::Template;

/// Common navigation bar of all entry editing pages
#[derive(Template)]
#[template(path = "sub_templates/entry_description.html")]
pub struct EntryDescriptionTemplate<'a> {
    entry: &'a FullEntry,
    rooms: &'a RoomByIdWithOrder<'a>,
    timezone: &'a chrono_tz::Tz,
}

impl<'a> EntryDescriptionTemplate<'a> {
    pub fn new(
        entry: &'a FullEntry,
        rooms: &'a RoomByIdWithOrder<'a>,
        timezone: &'a chrono_tz::Tz,
    ) -> Self {
        Self {
            entry,
            rooms,
            timezone,
        }
    }

    pub fn to_our_timezone(
        &self,
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) -> chrono::NaiveDateTime {
        timestamp.with_timezone(self.timezone).naive_local()
    }
}

impl askama::filters::HtmlSafe for EntryDescriptionTemplate<'_> {}

/// Filters for the askama template
mod filters {
    pub use crate::web::ui::askama_filters::markdown;
}
