use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{
    Category, Event, FullEntry, FullNewEntry, FullPreviousDate, NewEntry, PreviousDate, Room,
};
use crate::data_store::{EntryId, EventId, StoreError};
use crate::web::ui::base_template::{BaseTemplateContext, MainNavButton};
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashMessageActionButton, FlashType, FlashesInterface};
use crate::web::ui::form_values::{BoolFormValue, FormValue, _FormValidSimpleValidate};
use crate::web::ui::sub_templates;
use crate::web::ui::sub_templates::form_inputs::{
    CheckboxTemplate, FormFieldTemplate, HiddenInputTemplate, InputConfiguration, InputSize,
    InputType, SelectEntry, SelectTemplate,
};
use crate::web::ui::time_calculation::{
    get_effective_date, most_reasonable_date, timestamp_from_effective_date_and_time, TIME_ZONE,
};
use crate::web::ui::util::{event_days, url_for_entry_details};
use crate::web::ui::{time_calculation, util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html, Query, Redirect};
use actix_web::{get, post, web, Either, HttpRequest, HttpResponse, Responder};
use askama::Template;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeSet;
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
    let (entry, event, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageEntries)?;
        Ok((
            store.get_entry(&auth, entry_id)?,
            store.get_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
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
            event: Some(&event),
            current_date: Some(get_effective_date(&entry_begin)),
            auth_token: Some(&auth),
            active_main_nav_button: None,
        },
        event: &event,
        form_data: &form_data,
        rooms: &rooms,
        categories: &categories,
        entry_id: Some(&entry_id),
        has_unsaved_changes: false,
        is_new_entry: false,
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
        Some(entry_id),
    );

    let mut entry_begin = old_entry.entry.begin;
    let result = if let Some((mut entry, previous_last_updated, create_previous_date)) = entry {
        entry.entry.event_id = event_id;
        entry_begin = entry.entry.begin;
        if let Some(previous_date_comment) = create_previous_date {
            if entry.entry.begin != old_entry.entry.begin
                || entry.entry.end != old_entry.entry.end
                || !unordered_equality(&entry.room_ids, &old_entry.room_ids)
            {
                entry.previous_dates.push(FullPreviousDate {
                    previous_date: PreviousDate {
                        id: Uuid::now_v7(),
                        entry_id,
                        comment: previous_date_comment,
                        begin: old_entry.entry.begin,
                        end: old_entry.entry.end,
                    },
                    room_ids: old_entry.room_ids.clone(),
                });
            }
        }
        let auth_clone = auth.clone();
        let result = web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            store.create_or_update_entry(&auth_clone, entry, true, previous_last_updated)?;
            Ok(())
        })
        .await?;

        match result {
            Ok(()) => FormSubmitResult::Success,
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
            event: Some(&event),
            current_date: Some(get_effective_date(&old_entry.entry.begin)),
            auth_token: Some(&auth),
            active_main_nav_button: None,
        },
        event: &event,
        form_data: &data,
        rooms: &rooms,
        categories: &categories,
        entry_id: Some(&entry_id),
        has_unsaved_changes: true,
        is_new_entry: false,
    };

    create_edit_form_response(
        result,
        &tmpl,
        "Der Eintrag",
        req.url_for(
            "edit_entry_form",
            &[event_id.to_string(), entry_id.to_string()],
        )?,
        "edit_entry_form",
        false,
        url_for_entry_details(
            &req,
            event_id,
            &entry_id,
            &time_calculation::get_effective_date(&entry_begin),
        )?,
        &req,
    )
}

#[get("/{event_id}/new_entry")]
async fn new_entry_form(
    path: web::Path<i32>,
    query_data: Query<NewEntryQueryParams>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let date = query_data.date;
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;
    let store = state.store.clone();
    let (event, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageEntries)?;
        Ok((
            store.get_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let entry_id = Uuid::now_v7();
    let entry_date = date.unwrap_or_else(|| most_reasonable_date(&event));
    // TODO set default category
    let form_data = EntryFormData::for_new_entry(entry_id, entry_date);

    let tmpl = EditEntryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neuer Eintrag",
            event: Some(&event),
            current_date: date,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::AddEntry),
        },
        event: &event,
        form_data: &form_data,
        rooms: &rooms,
        categories: &categories,
        entry_id: Some(&entry_id),
        has_unsaved_changes: false,
        is_new_entry: true,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/new_entry")]
async fn new_entry(
    path: web::Path<i32>,
    query_data: Query<NewEntryQueryParams>,
    data: Form<EntryFormData>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let date = query_data.date;
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageEntries, event_id)?;
    let store = state.store.clone();
    let (event, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageEntries)?;
        Ok((
            store.get_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let mut data = data.into_inner();
    let entry = data.validate(
        &rooms.iter().map(|r| r.id).collect(),
        &categories.iter().map(|c| c.id).collect(),
        None,
    );

    let mut entry_id = None;
    let mut entry_begin = chrono::DateTime::<chrono::Utc>::default();
    let result = if let Some((mut entry, _, _)) = entry {
        let auth_clone = auth.clone();
        entry_id = Some(entry.entry.id);
        entry.entry.event_id = event_id;
        entry_begin = entry.entry.begin;
        let result = web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            // TODO detect and ignore double addition
            store.create_or_update_entry(&auth_clone, entry, false, None)?;
            Ok(())
        })
        .await?;

        match result {
            Ok(()) => FormSubmitResult::Success,
            Err(e) => match e {
                StoreError::TransactionConflict => FormSubmitResult::TransactionConflict,
                _ => FormSubmitResult::UnexpectedError(e.into()),
            },
        }
    } else {
        FormSubmitResult::ValidationError
    };

    let tmpl = EditEntryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neuer Eintrag",
            event: Some(&event),
            current_date: date,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::AddEntry),
        },
        event: &event,
        form_data: &data,
        rooms: &rooms,
        categories: &categories,
        entry_id: entry_id.as_ref(),
        has_unsaved_changes: true,
        is_new_entry: true,
    };

    create_edit_form_response(
        result,
        &tmpl,
        "Der Eintrag",
        req.url_for("new_entry_form", &[event_id.to_string()])?,
        "edit_entry_form",
        true,
        url_for_entry_details(
            &req,
            event_id,
            &entry_id.unwrap_or_default(),
            &time_calculation::get_effective_date(&entry_begin),
        )?,
        &req,
    )
}

/// Query parameters for the new_entry form.
#[derive(Deserialize, Serialize)]
pub struct NewEntryQueryParams {
    /// When given, used to pre-fill the date field of the new entry and to navigate back to this
    /// date when aborting.
    pub date: Option<chrono::NaiveDate>,
}

/// Helper type for representing the different possible outcomes of submitting the edit form.
///
/// They are used to delegate creating appropriate response to [create_edit_form_response()].
enum FormSubmitResult {
    Success,
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
    tmpl: impl Template,
    name_of_thing: &'static str,
    form_url: url::Url,
    form_name: &'static str,
    is_new_entity: bool,
    success_redirect: url::Url,
    request: &HttpRequest,
) -> Result<Either<Redirect, HttpResponse>, AppError> {
    match result {
        FormSubmitResult::Success => {
            request.add_flash_message(FlashMessage {
                flash_type: FlashType::Success,
                message: if is_new_entity {
                    format!("{} wurde gespeichert.", name_of_thing)
                } else {
                    "Änderung wurde gespeichert.".to_owned()
                },
                keep_open: false,
                button: None,
            });
            Ok(Either::Left(
                Redirect::to(success_redirect.to_string()).see_other(),
            ))
        }
        FormSubmitResult::ValidationError => {
            request.add_flash_message(FlashMessage {
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
            request.add_flash_message(FlashMessage {
                flash_type: FlashType::Error,
                message: format!("{} wurde zwischenzeitlich bearbeitet. Bitte das Formular neu laden und die Änderung erneut durchführen.", name_of_thing),
                keep_open: true,
                button: Some(FlashMessageActionButton::ReloadCleanForm {
                    form_url: form_url.to_string(),
                }),
            });
            Ok(Either::Right(HttpResponse::Conflict().body(tmpl.render()?)))
        }
        FormSubmitResult::TransactionConflict => {
            request.add_flash_message(FlashMessage {
                flash_type: FlashType::Warning,
                message: "Konnte wegen parallelem Datenbank-Zugriff nicht speichern. Bitte Formular erneut absenden."
                    .to_owned(),
                keep_open: true,
                button: Some(FlashMessageActionButton::SubmitForm { form_id: form_name.to_string() }),
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
    form_data: &'a EntryFormData,
    categories: &'a Vec<Category>,
    rooms: &'a Vec<Room>,
    entry_id: Option<&'a EntryId>,
    has_unsaved_changes: bool,
    is_new_entry: bool,
}

impl<'a> EditEntryFormTemplate<'a> {
    fn post_url(&self) -> Result<url::Url, AppError> {
        if self.is_new_entry {
            let mut url = self
                .base
                .request
                .url_for("new_entry", &[self.event.id.to_string()])?;
            url.set_query(Some(&serde_urlencoded::to_string(NewEntryQueryParams {
                date: self.base.current_date,
            })?));
            Ok(url)
        } else {
            Ok(self.base.request.url_for(
                "edit_entry",
                &[
                    self.event.id.to_string(),
                    self.entry_id
                        .expect("For non-new entries, `entry_id` should always be known.")
                        .to_string(),
                ],
            )?)
        }
    }
    fn abort_url(&self) -> Result<url::Url, actix_web::error::UrlGenerationError> {
        if self.is_new_entry {
            self.base.request.url_for(
                "main_list",
                &[
                    self.event.id.to_string(),
                    self.base
                        .current_date
                        .unwrap_or_else(|| time_calculation::most_reasonable_date(self.event))
                        .to_string(),
                ],
            )
        } else {
            url_for_entry_details(
                self.base.request,
                self.event.id,
                self.entry_id
                    .expect("For non-new entries, `entry_id` should always be known."),
                &self
                    .base
                    .current_date
                    .expect("For non-new entries, `date` should always be known."),
            )
        }
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
    /// Id of the entry, only used for creating new entries (for editing existing entries, the id is
    /// taken from the URL and passed to [validate] as `known_entry_id` instead)
    entry_id: FormValue<Uuid>,
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
    /// `last_updated` value of the (original) entry. Used for detecting editing conflicts.
    /// Only used for editing existing entries; can be empty/missing when creating new entries.
    last_updated: FormValue<validation::SimpleTimestampMicroseconds>,
    create_previous_date: BoolFormValue,
    previous_date_comment: FormValue<String>,
}

impl EntryFormData {
    fn for_new_entry(entry_id: EntryId, date: chrono::NaiveDate) -> Self {
        Self {
            entry_id: entry_id.into(),
            day: validation::IsoDate(date).into(),
            ..Self::default()
        }
    }

    fn validate(
        &mut self,
        rooms: &Vec<Uuid>,
        categories: &Vec<Uuid>,
        known_entry_id: Option<EntryId>,
    ) -> Option<(
        FullNewEntry,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
    )> {
        let entry_id = known_entry_id.or_else(|| self.entry_id.validate());
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
        let create_previous_date = self.create_previous_date.get_value();
        let previous_date_comment = self.previous_date_comment.validate();

        let begin = timestamp_from_effective_date_and_time(day?.into_inner(), time?.into_inner());
        Some((
            FullNewEntry {
                entry: NewEntry {
                    id: entry_id?,
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
            previous_last_updated.map(|v| v.0),
            if create_previous_date {
                previous_date_comment
            } else {
                None
            },
        ))
    }
}

impl From<FullEntry> for EntryFormData {
    fn from(value: FullEntry) -> Self {
        Self {
            entry_id: FormValue::empty(),
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
            create_previous_date: false.into(),
            previous_date_comment: "".to_string().into(),
        }
    }
}

fn unordered_equality<T: Eq + Ord>(a: &[T], b: &[T]) -> bool {
    // Source: https://stackoverflow.com/a/42748484/10315508
    let a: BTreeSet<_> = a.iter().collect();
    let b: BTreeSet<_> = b.iter().collect();

    a == b
}
