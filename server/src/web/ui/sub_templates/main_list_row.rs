use crate::data_store::models::{Category, FullEntry, FullPreviousDate, Room};
use crate::data_store::{CategoryId, RoomId};
use crate::web::ui::colors::CategoryColors;
use crate::web::ui::time_calculation::TIME_ZONE;
use crate::web::ui::util;
use actix_web::error::UrlGenerationError;
use actix_web::HttpRequest;
use askama::Template;
use std::collections::BTreeMap;

#[derive(Template)]
#[template(path = "sub_templates/main_list_row.html")]
pub struct MainListRowTemplate<'a> {
    request: &'a HttpRequest,
    row: &'a MainListRow<'a>,
    category: &'a Category,
    rooms: &'a BTreeMap<uuid::Uuid, &'a Room>,
    show_edit_links: bool,
    date_context: Option<chrono::NaiveDate>,
}

impl<'a> MainListRowTemplate<'a> {
    pub fn new(
        request: &'a HttpRequest,
        row: &'a MainListRow<'a>,
        entry_category: &'a Category,
        rooms: &'a BTreeMap<uuid::Uuid, &'a Room>,
        show_edit_links: bool,
        date_context: Option<chrono::NaiveDate>,
    ) -> Self {
        assert_eq!(row.entry.entry.category, entry_category.id);
        Self {
            request,
            row,
            category: entry_category,
            rooms,
            show_edit_links,
            date_context,
        }
    }

    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&TIME_ZONE).naive_local()
    }

    fn url_for_edit_entry(&self, entry: &FullEntry) -> Result<String, UrlGenerationError> {
        util::url_for_edit_entry(self.request, entry)
    }

    /// Generate the HTML 'class' attribute for the table row of the given `entry`
    fn css_class_for_tr(&self, row: &'a MainListRow<'a>) -> String {
        let mut result = css_class_for_category(&row.entry.entry.category);
        result.push_str(" kuea-with-category");
        if self.category.is_official {
            result.push_str(" fw-semibold");
        }
        if !row.entry_takes_place_now() {
            result.push_str(" kuea-cancelled");
        }
        if row.entry.entry.is_room_reservation {
            result.push_str(" fst-italic");
        }
        result
    }
}

/// A single row in the list view
///
/// This can either represent a KüA-Plan entry itself at its scheduled time or one or more
/// previous_dates of one (!) entry or a combination of both. The struct does not hold the data
/// itself but only contains references to the [FullEntry] struct and the relevant parts of it.
pub struct MainListRow<'a> {
    /// The KüA plan entry this row is about
    pub entry: &'a FullEntry,
    /// The relevant timestamp for sorting this row in the list. I.e. the `begin` of the entry or
    /// the relevant previous_date or the minimum of all of those (when this row covers more than
    /// one begin time)
    pub sort_time: &'a chrono::DateTime<chrono::Utc>,
    /// `true` if this list row represents the entry itself (with its currently scheduled date),
    /// maybe together with one or more previous dates. `false` if this list entry *only* represents
    /// previous_dates of the KüA-Plan entry
    pub includes_entry: bool,
    /// The previous_dates represented by this list row (if any)
    pub previous_dates: Vec<&'a FullPreviousDate>,
    /// The merged set of rooms of all dates represented by this list row
    pub merged_rooms: Vec<&'a RoomId>,
    /// The set of unique `(begin, end)` times represented by this row that are not equal to the
    /// entry's current scheduled time.
    pub additional_times: Vec<(
        &'a chrono::DateTime<chrono::Utc>,
        &'a chrono::DateTime<chrono::Utc>,
    )>,
}

impl<'a> MainListRow<'a> {
    /// Create a MainListEntry for given `entry` itself
    pub fn form_entry(entry: &'a FullEntry) -> Self {
        Self {
            entry,
            sort_time: &entry.entry.begin,
            includes_entry: true,
            previous_dates: vec![],
            merged_rooms: entry.room_ids.iter().collect(),
            additional_times: vec![],
        }
    }

    /// Create a MainListEntry for the given `previous_date` of the `entry`
    pub fn from_previous_date(entry: &'a FullEntry, previous_date: &'a FullPreviousDate) -> Self {
        debug_assert_eq!(previous_date.previous_date.entry_id, entry.entry.id);
        Self {
            entry,
            sort_time: &previous_date.previous_date.begin,
            includes_entry: false,
            previous_dates: vec![previous_date],
            merged_rooms: previous_date.room_ids.iter().collect(),
            additional_times: vec![(
                &previous_date.previous_date.begin,
                &previous_date.previous_date.end,
            )],
        }
    }

    /// Merge two MainListEntries of the same KüA-Plan `entry`.
    ///
    /// This merges all information from `other` into `self`, such that `self` represents all the
    /// dates of the entry (current or previous) of `other` as well, afterward.
    pub fn merge_from(&mut self, other: &MainListRow<'a>) {
        debug_assert_eq!(self.entry.entry.id, other.entry.entry.id);
        self.sort_time = std::cmp::min(self.sort_time, other.sort_time);
        self.includes_entry |= other.includes_entry;
        self.previous_dates.extend_from_slice(&other.previous_dates);
        for times in other.additional_times.iter() {
            if !self.additional_times.contains(&times)
                && *times != (&self.entry.entry.begin, &self.entry.entry.end)
            {
                self.additional_times.push(*times);
            }
        }
        for room in other.merged_rooms.iter() {
            if !self.merged_rooms.contains(&room) {
                self.merged_rooms.push(room);
            }
        }
    }

    /// Check if this row represents an entry taking place.
    /// This means that this row represents the entry itself and the entry is not cancelled.
    pub fn entry_takes_place_now(&self) -> bool {
        self.includes_entry && !self.entry.entry.is_cancelled
    }
}

/// Generate all required (inline) CSS stylesheet content for the given category.
///
/// This function must be called within the template once for every category that is used by an
/// entry on the page.
/// It generates CSS rules for the category's CSS class (according to [css_class_for_category])
/// that can be used for rendering entries belonging to that category.
pub fn styles_for_category(category: &Category) -> String {
    let colors = CategoryColors::from_base_color_hex(&category.color)
        .expect("Category color should be a valid HTML hex color string.");
    format!(
        ".{0}{{ {1} }}",
        css_class_for_category(&category.id),
        colors.as_css(),
    )
}

/// Return the CSS class name representing the Category with id `category_id`
fn css_class_for_category(category_id: &CategoryId) -> String {
    format!("category-{}", category_id)
}

mod filters {
    pub use crate::web::ui::askama_filters::ellipsis;
}
