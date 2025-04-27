use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, Event, FullEntry, Room};
use crate::data_store::EntryFilter;
use crate::web::ui::base_template::BaseTemplateContext;
use crate::web::ui::endpoints::main_list::filters::css_class_for_category;
use crate::web::ui::error::AppError;
use crate::web::ui::time_calculation::{
    get_effective_date, EFFECTIVE_BEGIN_OF_DAY, TIME_BLOCKS, TIME_ZONE,
};
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::error::UrlGenerationError;
use actix_web::web::Html;
use actix_web::{get, web, HttpRequest, Responder};
use askama::Template;
use chrono::TimeZone;
use std::collections::BTreeMap;

#[get("/{event_id}/list/{date}")]
async fn main_list(
    path: web::Path<(i32, chrono::NaiveDate)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, date) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ShowKueaPlan, event_id)?;
    let (event, entries, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        Ok((
            store.get_event(&auth, event_id)?,
            store.get_entries_filtered(&auth, event_id, date_to_filter(date))?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let title = date.format("%d.%m.").to_string();
    let tmpl = MainListTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: &title,
        },
        entry_blocks: sort_entries_into_blocks(&entries),
        entries_with_descriptions: entries
            .iter()
            .filter(|e| !e.entry.is_cancelled && !e.entry.description.is_empty())
            .collect(),
        rooms: rooms.iter().map(|r| (r.id, r)).collect(),
        categories: categories.iter().map(|r| (r.id, r)).collect(),
        timezone: TIME_ZONE,
        date,
        event: &event,
        event_days: util::event_days(&event),
        user_can_edit_entries: auth.has_privilege(event_id, Privilege::ManageEntries),
    };
    Ok(Html::new(tmpl.render()?))
}

#[derive(Template)]
#[template(path = "main_list.html")]
struct MainListTemplate<'a> {
    base: BaseTemplateContext<'a>,
    entry_blocks: Vec<(String, Vec<&'a FullEntry>)>,
    entries_with_descriptions: Vec<&'a FullEntry>,
    rooms: BTreeMap<uuid::Uuid, &'a Room>,
    categories: BTreeMap<uuid::Uuid, &'a Category>,
    timezone: chrono_tz::Tz,
    date: chrono::NaiveDate,
    event: &'a Event,
    event_days: Vec<chrono::NaiveDate>,
    user_can_edit_entries: bool,
}

impl<'a> MainListTemplate<'a> {
    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&self.timezone).naive_local()
    }

    fn url_for_main_list(&self, date: &chrono::NaiveDate) -> Result<String, UrlGenerationError> {
        util::url_for_main_list(self.base.request, self.event.id, date)
    }

    fn url_for_edit_entry(&self, entry: &FullEntry) -> Result<String, UrlGenerationError> {
        util::url_for_edit_entry(self.base.request, entry)
    }

    fn css_class_for_entry(&self, entry: &'a FullEntry) -> String {
        let mut result = css_class_for_category(&entry.entry.category)
            .expect("CSS class calculation cannot fail");
        result.push_str(" kuea-with-category");
        if entry.entry.is_cancelled {
            result.push_str(" kuea-cancelled");
        }
        if entry.entry.is_room_reservation {
            result.push_str(" fst-italic");
        }
        result
    }
}

/// Generate an EntryFilter for retrieving only the entries on the given day (using the
/// EFFECTIVE_BEGIN_OF_DAY)
fn date_to_filter(date: chrono::NaiveDate) -> EntryFilter {
    let begin = date.and_time(EFFECTIVE_BEGIN_OF_DAY);
    let end = begin + chrono::Duration::days(1);
    let mut filter = EntryFilter::builder();
    filter.after(
        TIME_ZONE
            .from_local_datetime(&begin)
            .earliest()
            .map(|dt| dt.to_utc())
            .unwrap_or(begin.and_utc()),
    );
    filter.before(
        TIME_ZONE
            .from_local_datetime(&end)
            .latest()
            .map(|dt| dt.to_utc())
            .unwrap_or(end.and_utc()),
    );
    filter.build()
}

// entries must be sorted by begin timestamp
fn sort_entries_into_blocks(entries: &Vec<FullEntry>) -> Vec<(String, Vec<&FullEntry>)> {
    let mut result = Vec::new();
    let mut block_entries = Vec::new();
    let mut time_block_iter = TIME_BLOCKS.iter();
    let (mut time_block_name, mut time_block_time) = time_block_iter
        .next()
        .expect("At least one time block should be defined.");
    for entry in entries {
        while time_block_time.is_some_and(|block_begin_time| {
            entry.entry.begin.with_timezone(&TIME_ZONE).time() >= block_begin_time
        }) {
            // TODO convert to local timezone
            if !block_entries.is_empty() {
                result.push((time_block_name.to_string(), block_entries));
            }
            (time_block_name, time_block_time) = *time_block_iter
                .next()
                .expect("last time block's time must be 'None' to stop iteration");
            block_entries = Vec::new();
        }
        block_entries.push(entry);
    }
    if !block_entries.is_empty() {
        result.push((time_block_name.to_string(), block_entries));
    }
    result
}

/// Filters for the rinja template
mod filters {
    use crate::data_store::models::Category;
    use crate::data_store::CategoryId;
    use crate::web::ui::colors::CategoryColors;
    use chrono::{Datelike, Weekday};

    pub fn markdown(input: &str) -> askama::Result<askama::filters::Safe<String>> {
        let arena = comrak::Arena::new();
        let options = comrak::ComrakOptions {
            extension: comrak::ExtensionOptions::builder()
                .strikethrough(true)
                .tagfilter(true)
                .table(true)
                .footnotes(true)
                .underline(true)
                .build(),
            parse: Default::default(),
            render: comrak::RenderOptions::builder().escape(true).build(),
        };
        let ast_root = comrak::parse_document(&arena, input, &options);

        markdown_increase_heading_level(ast_root, 3);

        let mut bw = std::io::BufWriter::new(Vec::new());
        comrak::format_html(ast_root, &options, &mut bw)?;
        Ok(askama::filters::Safe(
            String::from_utf8(
                bw.into_inner()
                    .expect("Extracting vector from BufWriter should not fail."),
            )
            .expect("comrak HTML formatter should only generate valid UTF-8 bytes."),
        ))
    }

    fn markdown_increase_heading_level<'a>(
        ast_root: &'a comrak::nodes::AstNode<'a>,
        increase_by: u8,
    ) {
        for node in ast_root.descendants() {
            if let comrak::nodes::NodeValue::Heading(ref mut heading) = node.data.borrow_mut().value
            {
                heading.level = (heading.level + increase_by).clamp(1, 6);
            }
        }
    }

    pub fn weekday(date: &chrono::NaiveDate) -> askama::Result<&'static str> {
        Ok(match date.weekday() {
            Weekday::Mon => "Montag",
            Weekday::Tue => "Dienstag",
            Weekday::Wed => "Mittwoch",
            Weekday::Thu => "Donnerstag",
            Weekday::Fri => "Freitag",
            Weekday::Sat => "Samstag",
            Weekday::Sun => "Sonntag",
        })
    }

    pub fn weekday_short(date: &chrono::NaiveDate) -> askama::Result<&'static str> {
        Ok(match date.weekday() {
            Weekday::Mon => "Mo",
            Weekday::Tue => "Di",
            Weekday::Wed => "Mi",
            Weekday::Thu => "Do",
            Weekday::Fri => "Fr",
            Weekday::Sat => "Sa",
            Weekday::Sun => "So",
        })
    }

    pub fn styles_for_category(category: &Category) -> askama::Result<String> {
        let colors = CategoryColors::from_base_color_hex(&category.color)
            .expect("Category color should be a valid HTML hex color string.");
        Ok(format!(
            ".{0}{{ {1} }}",
            css_class_for_category(&category.id)?,
            colors.as_css(),
        ))
    }

    pub fn css_class_for_category(category_id: &CategoryId) -> askama::Result<String> {
        Ok(format!("category-{}", category_id))
    }

    pub fn ellipsis(value: &str, length: usize) -> askama::Result<String> {
        if value.chars().count() > length {
            Ok(format!(
                "{}…",
                value.chars().take(length - 1).collect::<String>()
            ))
        } else {
            Ok(value.to_owned())
        }
    }
}
