use crate::auth_session::SessionToken;
use crate::data_store::models::{Event, FullEntry};
use crate::data_store::EntryId;
use crate::web::ui::forms::{BoolFormValue, FormValue, InputSize, InputType, SelectEntry};
use crate::web::ui::{AppError, BaseTemplateContext};
use crate::web::AppState;
use actix_web::web::Html;
use actix_web::{get, post, web, HttpRequest, Responder};
use rinja::Template;
use serde::Deserialize;
use std::borrow::Cow;

#[get("/{event_id}/entry/{entry_id}/edit")]
async fn edit_entry_form(
    path: web::Path<(i32, EntryId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, entry_id) = path.into_inner();
    let session_token = SessionToken::from_string(
        req.cookie("kuea-plan-session")
            .ok_or(AppError::NoSession)?
            .value(),
        &state.secret,
        super::SESSION_COOKIE_MAX_AGE,
    )?;
    let (event, entry, rooms, categories) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        Ok((
            store.get_event(&auth, event_id)?,
            store.get_entry(&auth, entry_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
        ))
    })
    .await??;

    let tmpl = EditEntryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            event_id,
            page_title: "Eintrag bearbeiten", // TODO
        },
        event: &event,
        form_data: &entry.into(),
        post_url: req.url_for("edit_entry", &[event_id.to_string(), entry_id.to_string()])?,
        category_entries: &categories
            .iter()
            .map(|c| SelectEntry {
                value: Cow::Owned(c.id.to_string()),
                label: Cow::Borrowed(&c.title),
            })
            .collect(),
    };
    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/entry/{entry_id}/edit")]
async fn edit_entry(
    path: web::Path<(i32, EntryId)>,
    data: web::Form<EntryFormData>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    // TODO
    Ok("")
}

#[derive(Template)]
#[template(path = "edit_entry_form.html")]
struct EditEntryFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    event: &'a Event,
    form_data: &'a EntryFormData,
    post_url: url::Url,
    category_entries: &'a Vec<SelectEntry<'a>>,
}

#[derive(Default, Deserialize)]
struct EntryFormData {
    title: FormValue,
    comment: FormValue,
    room_comment: FormValue,
    time_comment: FormValue,
    description: FormValue,
    category: FormValue,
    is_cancelled: BoolFormValue,
    is_room_reservation: BoolFormValue,
}

impl From<FullEntry> for EntryFormData {
    fn from(value: FullEntry) -> Self {
        Self {
            title: value.entry.title.into(),
            comment: value.entry.comment.into(),
            room_comment: value.entry.room_comment.into(),
            time_comment: value.entry.time_comment.into(),
            description: value.entry.description.into(),
            category: value.entry.category.into(),
            is_cancelled: value.entry.is_cancelled.into(),
            is_room_reservation: value.entry.is_room_reservation.into(),
        }
    }
}
