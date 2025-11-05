use crate::data_store::models::{Category, EventClockInfo, FullEntry, FullPreviousDate, Room};
use crate::data_store::{CategoryId, RoomId};
use crate::web::time_calculation;
use crate::web::ui::colors::CategoryColors;
use crate::web::ui::util;
use crate::web::ui::util::url_for_entry_details;
use actix_web::error::UrlGenerationError;
use actix_web::HttpRequest;
use askama::Template;
use std::collections::BTreeMap;

/// Type of the room lookup map as expected by the MainListRowTemplate.
///
/// Consists of a BTreeMap that maps each room's uuid to the room (as reference) and a sort key,
/// used for sorting the looked-up rooms back to their original order.
pub struct RoomByIdWithOrder<'a>(BTreeMap<uuid::Uuid, (&'a Room, usize)>);

impl<'a, 'b> RoomByIdWithOrder<'a> {
    pub fn iter_rooms_by_id_ordered<I: IntoIterator<Item = &'b RoomId>>(
        &'a self,
        ids: I,
    ) -> impl Iterator<Item = &'a Room> {
        let mut rooms: Vec<_> = ids.into_iter().filter_map(|id| self.0.get(id)).collect();
        rooms.sort_by_key(|(_room, sort_key)| *sort_key);
        rooms.into_iter().map(|(room, _sort_key)| *room)
    }
}

impl<'a> FromIterator<&'a Room> for RoomByIdWithOrder<'a> {
    fn from_iter<T: IntoIterator<Item = &'a Room>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .enumerate()
                .map(|(idx, r)| (r.id, (r, idx)))
                .collect(),
        )
    }
}

/// Sub-Template for rendering a single row in a main KüA-List, based on a [MainListRow] struct.
///
/// The output of this template must be used within a `<table class="table kuealist">` with four
/// columns 'title', 'time', 'place', 'people'.
///
/// An instance of this template can be created with [new].
#[derive(Template)]
#[template(path = "sub_templates/main_list_row.html")]
pub struct MainListRowTemplate<'a> {
    request: &'a HttpRequest,
    row: &'a MainListRow<'a>,
    category: &'a Category,
    rooms: &'a RoomByIdWithOrder<'a>,
    clock_info: &'a EventClockInfo,
    show_edit_links: bool,
    show_description_links: bool,
    date_context: Option<chrono::NaiveDate>,
    room_context: Option<uuid::Uuid>,
    main_entry_link_mode: MainEntryLinkMode,
}

impl<'a> MainListRowTemplate<'a> {
    pub fn new(
        request: &'a HttpRequest,
        row: &'a MainListRow<'a>,
        entry_category: &'a Category,
        rooms: &'a RoomByIdWithOrder<'a>,
        clock_info: &'a EventClockInfo,
        show_edit_links: bool,
        show_description_links: bool,
        date_context: Option<chrono::NaiveDate>,
        room_context: Option<uuid::Uuid>,
        main_entry_link_mode: MainEntryLinkMode,
    ) -> Self {
        assert_eq!(row.entry.entry.category, entry_category.id);
        Self {
            request,
            row,
            category: entry_category,
            rooms,
            clock_info,
            show_edit_links,
            show_description_links,
            date_context,
            room_context,
            main_entry_link_mode,
        }
    }

    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp
            .with_timezone(&self.clock_info.timezone)
            .naive_local()
    }

    /// Helper function to retrieve Room references for the currently planned rooms of the entry,
    /// ordered by the sort key.
    fn get_entry_rooms_ordered(&self) -> impl Iterator<Item = &Room> {
        self.rooms
            .iter_rooms_by_id_ordered(self.row.entry.room_ids.iter())
    }

    /// Helper function to retrieve Room references for the previously planned rooms of the entry,
    /// i.e. rooms of previous dates represented by this row which are not currently planned for the
    /// entry.
    fn get_previous_rooms_ordered(&self) -> impl Iterator<Item = &Room> {
        self.rooms.iter_rooms_by_id_ordered(
            self.row
                .merged_rooms
                .iter()
                .filter(|id| !self.row.includes_entry || !self.row.entry.room_ids.contains(id))
                .map(|r| *r),
        )
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

    /// Generate a URL that takes the user directly to the current kueaplan entry date in the
    /// relevant list, according to main_entry_link_mode, if possible.
    pub fn url_for_current_entry(&self) -> Result<Option<url::Url>, UrlGenerationError> {
        let entry = &self.row.entry.entry;
        match self.main_entry_link_mode {
            MainEntryLinkMode::None => Ok(None),
            MainEntryLinkMode::ByDate => Some(url_for_entry_details(
                self.request,
                entry.event_id,
                &entry.id,
                &time_calculation::get_effective_date(&entry.begin, self.clock_info),
            ))
            .transpose(),
            MainEntryLinkMode::ByCategory => {
                let mut url = self.request.url_for(
                    "main_list_by_category",
                    [&entry.event_id.to_string(), &entry.category.to_string()],
                )?;
                url.set_fragment(Some(&format!("entry-{}", entry.id)));
                Ok(Some(url))
            }
            MainEntryLinkMode::ByRoomContext => {
                if let Some(room_id) = self.room_context {
                    if self.row.entry.room_ids.contains(&room_id) {
                        let mut url = self.request.url_for(
                            "main_list_by_room",
                            [&entry.event_id.to_string(), &room_id.to_string()],
                        )?;
                        url.set_fragment(Some(&format!("entry-{}", entry.id)));
                        Ok(Some(url))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
        }
    }
}

impl askama::filters::HtmlSafe for MainListRowTemplate<'_> {}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum MainEntryLinkMode {
    None,
    ByDate,
    ByCategory,
    ByRoomContext,
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
    /// The set of unique `(begin, end)` times represented by this row
    pub merged_times: Vec<(
        &'a chrono::DateTime<chrono::Utc>,
        &'a chrono::DateTime<chrono::Utc>,
    )>,
    pub is_first_row_of_next_calendar_date: bool,
}

impl<'a> MainListRow<'a> {
    /// Create a MainListEntry for given `entry` itself
    pub fn from_entry(entry: &'a FullEntry) -> Self {
        Self {
            entry,
            sort_time: &entry.entry.begin,
            includes_entry: true,
            previous_dates: vec![],
            merged_rooms: entry.room_ids.iter().collect(),
            merged_times: vec![(&entry.entry.begin, &entry.entry.end)],
            is_first_row_of_next_calendar_date: false,
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
            merged_times: vec![(
                &previous_date.previous_date.begin,
                &previous_date.previous_date.end,
            )],
            is_first_row_of_next_calendar_date: false,
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
        for times in other.merged_times.iter() {
            if !self.merged_times.contains(times) {
                self.merged_times.push(*times);
            }
        }
        for room in other.merged_rooms.iter() {
            if !self.merged_rooms.contains(room) {
                self.merged_rooms.push(room);
            }
        }
        self.is_first_row_of_next_calendar_date |= other.is_first_row_of_next_calendar_date;
    }

    /// Check if this row represents an entry taking place.
    /// This means that this row represents the entry itself and the entry is not cancelled.
    pub fn entry_takes_place_now(&self) -> bool {
        self.includes_entry && !self.entry.entry.is_cancelled
    }

    fn rooms_differ_from_entry(&self) -> bool {
        // According to https://stackoverflow.com/a/64227550/10315508 this is faster than building
        // a Set for small vectors – as expected.
        !(self
            .entry
            .room_ids
            .iter()
            .all(|r| self.merged_rooms.contains(&r))
            && self
                .merged_rooms
                .iter()
                .all(|r| self.entry.room_ids.contains(*r)))
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
