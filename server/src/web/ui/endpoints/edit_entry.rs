use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Category, Event, FullEntry, FullNewEntry, NewEntry, Room};
use crate::data_store::{EntryId, EventId, StoreError};
use crate::web::ui::base_template::BaseTemplateContext;
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashMessageActionButton, FlashType, FlashesInterface};
use crate::web::ui::forms::{
    BoolFormValue, FormValue, InputSize, InputType, SelectEntry, _FormValidSimpleValidate,
};
use crate::web::ui::time_calculation::{
    get_effective_date, timestamp_from_effective_date_and_time, TIME_ZONE,
};
use crate::web::ui::util::{event_days, url_for_entry};
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html, Redirect};
use actix_web::{get, post, web, Either, HttpRequest, HttpResponse, Responder};
use askama::Template;
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
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;
    let store = state.store.clone();
    let (entry, event, rooms, categories) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageEntries)?;
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
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;
    let store = state.store.clone();
    let (event, old_entry, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageEntries)?;
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

    let result = if let Some((mut entry, privious_last_updated)) = entry {
        entry.entry.id = entry_id;
        entry.entry.event_id = event_id;
        let entry_begin = entry.entry.begin;
        let result = web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            store.create_or_update_entry(&auth, entry, true, Some(privious_last_updated))?;
            Ok(())
        })
        .await?;
        // TODO allow creating new previous_date

        match result {
            Ok(()) => FormSubmitResult::Success { entry_begin },
            Err(e) => match e {
                StoreError::TransactionConflict => FormSubmitResult::TransactionConflict,
                StoreError::ConcurrentEditConflict => FormSubmitResult::ConcurrentEditConflict,
                _ => FormSubmitResult::UnexpectedError(e.into()),
            },
        }
    } else {
        FormSubmitResult::ValidationError
    };

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

    create_edit_form_response(result, event_id, tmpl)
}

/// Helper type for representing the different possible outcomes of submitting the edit form.
///
/// They are used to delegate creating appropriate response to [create_edit_form_response()].
enum FormSubmitResult {
    Success {
        entry_begin: chrono::DateTime<chrono::Utc>,
    },
    ValidationError,
    TransactionConflict,
    ConcurrentEditConflict,
    UnexpectedError(AppError),
}

/// Helper function for generating the HTTP response in [edit_entry()].
///
/// Together with the [FormSubmitResult] helper type, this function helps keeping the code of
/// edit_entry() more readable. Without these tricks we'd have error message creation functions
/// scattered all over the code.
fn create_edit_form_response(
    result: FormSubmitResult,
    event_id: EventId,
    tmpl: EditEntryFormTemplate,
) -> Result<impl Responder, AppError> {
    match result {
        FormSubmitResult::Success { entry_begin } => {
            tmpl.base.request.add_flash_message(FlashMessage {
                flash_type: FlashType::Success,
                message: "Änderung wurde gespeichert.".to_owned(),
                keep_open: false,
                button: None,
            });
            Ok(Either::Left(
                Redirect::to(
                    url_for_entry(&tmpl.base.request, event_id, &tmpl.entry_id, &entry_begin)?
                        .to_string(),
                )
                .see_other(),
            ))
        }
        FormSubmitResult::ValidationError => {
            tmpl.base.request.add_flash_message(FlashMessage {
                flash_type: FlashType::Error,
                message: "Eingegebene Daten sind ungültig. Bitte markierte Felder überprüfen."
                    .to_owned(),
                keep_open: false,
                button: None,
            });
            Ok(Either::Right(
                HttpResponse::UnprocessableEntity().body(tmpl.render()?),
            ))
        }
        FormSubmitResult::ConcurrentEditConflict => {
            tmpl.base.request.add_flash_message(FlashMessage {
                flash_type: FlashType::Error,
                message: "Der Eintrag wurde zwischenzeitlich bearbeitet. Bitte das Formular neu laden und die Änderung erneut durchführen."
                    .to_owned(),
                keep_open: true,
                button: Some(FlashMessageActionButton::ReloadCleanForm {
                    form_url: tmpl.base.request.url_for("edit_entry_form",
                                                        &[event_id.to_string(), tmpl.entry_id.to_string()])?
                        .to_string()})
            });
            Ok(Either::Right(HttpResponse::Conflict().body(tmpl.render()?)))
        }
        FormSubmitResult::TransactionConflict => {
            tmpl.base.request.add_flash_message(FlashMessage {
                flash_type: FlashType::Warning,
                message: "Konnte wegen parallelem Datenbank-Zugriff nicht speichern. Bitte Formular erneut absenden."
                    .to_owned(),
                keep_open: true,
                button: Some(FlashMessageActionButton::SubmitForm {form_id: "edit_entry_form".to_string()})
            });
            Ok(Either::Right(
                HttpResponse::ServiceUnavailable().body(tmpl.render()?),
            ))
        }
        FormSubmitResult::UnexpectedError(e) => Err(e),
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
    title: FormValue<validation::NonEmptyString>,
    comment: FormValue<String>,
    room_comment: FormValue<String>,
    time_comment: FormValue<String>,
    description: FormValue<String>,
    responsible_person: FormValue<String>,
    day: FormValue<validation::IsoDate>,
    begin: FormValue<validation::TimeOfDay>,
    duration: FormValue<validation::NiceDurationHours>,
    category: FormValue<validation::UuidFromList>,
    rooms: FormValue<validation::CommaSeparatedUuidsFromList>,
    is_cancelled: BoolFormValue,
    is_room_reservation: BoolFormValue,
    is_exclusive: BoolFormValue,
    last_updated: FormValue<validation::SimpleTimestampMicroseconds>,
}

impl EntryFormData {
    fn validate(
        &mut self,
        rooms: &Vec<Uuid>,
        categories: &Vec<Uuid>,
    ) -> Option<(FullNewEntry, chrono::DateTime<chrono::Utc>)> {
        let title = self.title.validate();
        let comment = self.comment.validate();
        let time_comment = self.time_comment.validate();
        let room_comment = self.room_comment.validate();
        let description = self.description.validate();
        let responsible_person = self.responsible_person.validate();
        let is_cancelled = self.is_cancelled.get_value();
        let is_room_reservation = self.is_room_reservation.get_value();
        let is_exclusive = self.is_exclusive.get_value();
        let category = self.category.validate_with(categories);
        let room_ids = self.rooms.validate_with(rooms);
        let day = self.day.validate();
        let time = self.begin.validate();
        let duration = self.duration.validate();
        let previous_last_updated = self.last_updated.validate();

        let begin = timestamp_from_effective_date_and_time(day?.into_inner(), time?.into_inner());
        Some((
            FullNewEntry {
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
            },
            previous_last_updated?.0,
        ))
    }
}

impl From<FullEntry> for EntryFormData {
    fn from(value: FullEntry) -> Self {
        Self {
            title: validation::NonEmptyString(value.entry.title).into(),
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
            category: validation::UuidFromList(value.entry.category).into(),
            rooms: validation::CommaSeparatedUuidsFromList(value.room_ids).into(),
            is_cancelled: value.entry.is_cancelled.into(),
            is_room_reservation: value.entry.is_room_reservation.into(),
            is_exclusive: value.entry.is_exclusive.into(),
            last_updated: validation::SimpleTimestampMicroseconds(value.entry.last_updated).into(),
        }
    }
}
