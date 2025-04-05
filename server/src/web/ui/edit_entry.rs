use super::framework::forms::{BoolFormValue, FormValue, InputSize, InputType, SelectEntry};
use super::util::{
    event_days, get_effective_date, timestamp_from_effective_date_and_time, url_for_entry,
    TIME_ZONE,
};
use crate::auth_session::SessionToken;
use crate::data_store::models::{Category, Event, FullEntry, FullNewEntry, NewEntry, Room};
use crate::data_store::EntryId;
use crate::web::ui::framework::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::{framework::validation, AppError, BaseTemplateContext};
use crate::web::AppState;
use actix_web::web::{Form, Html, Redirect};
use actix_web::{get, post, web, Either, HttpRequest, HttpResponse, Responder};
use chrono::DateTime;
use rinja::Template;
use serde::Deserialize;
use std::borrow::Cow;
use uuid::Uuid;

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
    let store = state.store.clone();
    let (entry, event, rooms, categories) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        Ok((
            store.get_entry(&auth, entry_id)?,
            store.get_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
        ))
    })
    .await??;

    let entry_id = entry.entry.id;
    let entry_begin = entry.entry.begin;
    let form_data: EntryFormData = entry.into();

    let tmpl = EditEntryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Eintrag bearbeiten", // TODO
        },
        event: &event,
        entry_id: &entry_id,
        form_data: &form_data,
        rooms: &rooms,
        categories: &categories,
        entry_begin: &entry_begin,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/entry/{entry_id}/edit")]
async fn edit_entry(
    path: web::Path<(i32, EntryId)>,
    data: Form<EntryFormData>,
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
    let store = state.store.clone();
    let (event, old_entry, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.check_authorization(&session_token, event_id)?;
        Ok((
            store.get_event(&auth, event_id)?,
            store.get_entry(&auth, entry_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;
    if event_id != old_entry.entry.event_id {
        return Err(AppError::EntityNotFound);
    }

    let mut data = data.into_inner();
    let entry = data.validate(
        &rooms.iter().map(|r| r.id).collect(),
        &categories.iter().map(|c| c.id).collect(),
    );

    if let Some(mut entry) = entry {
        entry.entry.id = entry_id;
        entry.entry.event_id = event_id;
        let entry_begin = entry.entry.begin;
        web::block(move || -> Result<_, AppError> {
            let mut store = state.store.get_facade()?;
            store.create_or_update_entry(&auth, entry, true)?;
            Ok(())
        })
        .await??;
        // TODO if storing entry failed, show flash notification and show form again

        // TODO allow creating new previous_date
        req.add_flash_message(FlashMessage {
            flash_type: FlashType::SUCCESS,
            message: "Änderung wurde gespeichert.".to_owned(),
        });
        Ok(Either::Left(
            Redirect::to(url_for_entry(&req, event_id, &entry_id, &entry_begin)?.to_string())
                .see_other(),
        ))
    } else {
        req.add_flash_message(FlashMessage {
            flash_type: FlashType::ERROR,
            message: "Validierung fehlgeschlagen.".to_owned(),
        });
        let tmpl = EditEntryFormTemplate {
            base: BaseTemplateContext {
                request: &req,
                page_title: "Eintrag bearbeiten", // TODO
            },
            event: &event,
            entry_id: &entry_id,
            form_data: &data,
            rooms: &rooms,
            categories: &categories,
            entry_begin: &old_entry.entry.begin,
        };
        Ok(Either::Right(
            HttpResponse::UnprocessableEntity().body(tmpl.render()?),
        ))
    }
}

#[derive(Template)]
#[template(path = "edit_entry_form.html")]
struct EditEntryFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    event: &'a Event,
    entry_id: &'a EntryId,
    form_data: &'a EntryFormData,
    categories: &'a Vec<Category>,
    rooms: &'a Vec<Room>,
    entry_begin: &'a chrono::DateTime<chrono::Utc>,
}

impl<'a> EditEntryFormTemplate<'a> {
    fn post_url(&self) -> Result<url::Url, actix_web::error::UrlGenerationError> {
        self.base.request.url_for(
            "edit_entry",
            &[self.event.id.to_string(), self.entry_id.to_string()],
        )
    }
    fn abort_url(&self) -> Result<url::Url, actix_web::error::UrlGenerationError> {
        url_for_entry(
            self.base.request,
            self.event.id,
            self.entry_id,
            self.entry_begin,
        )
    }
    fn room_entries(&self) -> Vec<SelectEntry<'a>> {
        self.rooms
            .iter()
            .map(|r| SelectEntry {
                value: Cow::Owned(r.id.to_string()),
                text: Cow::Borrowed(&r.title),
            })
            .collect()
    }
    fn category_entries(&self) -> Vec<SelectEntry<'a>> {
        self.categories
            .iter()
            .map(|c| SelectEntry {
                value: Cow::Owned(c.id.to_string()),
                text: Cow::Borrowed(&c.title),
            })
            .collect()
    }
    fn day_entries(&self) -> Vec<SelectEntry<'static>> {
        event_days(self.event)
            .iter()
            .map(|date| SelectEntry {
                value: Cow::Owned(date.to_string()),
                text: Cow::Owned(date.format("%d.%m.").to_string()),
            })
            .collect()
    }
}

#[derive(Default, Deserialize, Debug)]
struct EntryFormData {
    title: FormValue,
    comment: FormValue,
    room_comment: FormValue,
    time_comment: FormValue,
    description: FormValue,
    responsible_person: FormValue,
    day: FormValue,
    begin: FormValue,
    duration: FormValue,
    category: FormValue,
    rooms: FormValue,
    is_cancelled: BoolFormValue,
    is_room_reservation: BoolFormValue,
    is_exclusive: BoolFormValue,
}

impl EntryFormData {
    fn validate(&mut self, rooms: &Vec<Uuid>, categories: &Vec<Uuid>) -> Option<FullNewEntry> {
        let title: Option<validation::NonEmptyString> = self.title.validate();
        let comment: Option<String> = self.comment.validate();
        let time_comment: Option<String> = self.time_comment.validate();
        let room_comment: Option<String> = self.room_comment.validate();
        let description: Option<String> = self.description.validate();
        let responsible_person: Option<String> = self.responsible_person.validate();
        let is_cancelled: bool = self.is_cancelled.get_value();
        let is_room_reservation: bool = self.is_room_reservation.get_value();
        let is_exclusive: bool = self.is_exclusive.get_value();
        let category: Option<validation::UuidFromList> = self.category.validate_with(categories);
        let room_ids: Option<validation::CommaSeparatedUuidsFromList> =
            self.rooms.validate_with(rooms);
        let day: Option<validation::IsoDate> = self.day.validate();
        let time: Option<validation::TimeOfDay> = self.begin.validate();
        let duration: Option<validation::NiceDurationHours> = self.duration.validate();

        let begin = timestamp_from_effective_date_and_time(day?.into_inner(), time?.into_inner());
        Some(FullNewEntry {
            entry: NewEntry {
                id: Default::default(),
                title: title?.into_inner(),
                description: description?,
                responsible_person: responsible_person?,
                is_room_reservation,
                event_id: 0,
                begin,
                end: begin + duration?.into_inner(),
                category: category?.into_inner(),
                comment: comment?,
                time_comment: time_comment?,
                room_comment: room_comment?,
                is_exclusive,
                is_cancelled,
            },
            room_ids: room_ids?.into_inner(),
            previous_dates: vec![],
        })
    }
}

impl From<FullEntry> for EntryFormData {
    fn from(value: FullEntry) -> Self {
        Self {
            title: value.entry.title.into(),
            comment: value.entry.comment.into(),
            room_comment: value.entry.room_comment.into(),
            time_comment: value.entry.time_comment.into(),
            description: value.entry.description.into(),
            responsible_person: value.entry.responsible_person.into(),
            day: validation::IsoDate(get_effective_date(&value.entry.begin)).into(),
            begin: validation::TimeOfDay(
                value
                    .entry
                    .begin
                    .with_timezone(&TIME_ZONE)
                    .naive_local()
                    .time(),
            )
            .into(),
            duration: validation::NiceDurationHours(value.entry.end - value.entry.begin).into(),
            category: value.entry.category.into(),
            rooms: validation::CommaSeparatedUuidsFromList(value.room_ids).into(),
            is_cancelled: value.entry.is_cancelled.into(),
            is_room_reservation: value.entry.is_room_reservation.into(),
            is_exclusive: value.entry.is_exclusive.into(),
        }
    }
}
