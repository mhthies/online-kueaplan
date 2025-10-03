use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{
    Category, ExtendedEvent, FullEntry, FullNewEntry, FullPreviousDate, PreviousDate, Room,
};
use crate::data_store::{EntryId, EventId, StoreError};
use crate::web::time_calculation::{get_effective_date, timestamp_from_effective_date_and_time};
use crate::web::ui::base_template::{AnyEventData, BaseTemplateContext, MainNavButton};
use crate::web::ui::error::AppError;
use crate::web::ui::form_values::{FormValue, _FormValidSimpleValidate};
use crate::web::ui::sub_templates::edit_entry_helpers::{
    EditEntryNavbar, EditEntryNavbarActiveLink,
};
use crate::web::ui::sub_templates::form_inputs::{
    FormFieldTemplate, HiddenInputTemplate, InputConfiguration, InputSize, InputType, SelectEntry,
    SelectTemplate,
};
use crate::web::ui::sub_templates::main_list_row::{
    styles_for_category, MainEntryLinkMode, MainListRow, MainListRowTemplate,
};
use crate::web::ui::util::{event_days, weekday_short};
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::BTreeMap;
use uuid::Uuid;

#[get("/{event_id}/entry/{entry_id}/new_previous_date")]
pub async fn new_previous_date_form(
    path: web::Path<(EventId, EntryId)>,
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
            store.get_extended_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?, // TODO only get relevant category?
            auth,
        ))
    })
    .await??;

    let previous_date_id = Uuid::now_v7();
    let form_data: PreviousDateFormData =
        PreviousDateFormData::for_new_previous_date(previous_date_id, &entry, &event);

    let tmpl = NewPreviousDateFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neuer Vorheriger Termin", // TODO
            event: AnyEventData::ExtendedEvent(&event),
            current_date: Some(get_effective_date(&entry.entry.begin, &event)),
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::ByDate),
        },
        event: &event,
        entry: &entry,
        rooms: &rooms,
        rooms_by_id: rooms.iter().map(|r| (r.id, r)).collect(),
        entry_category: categories
            .iter()
            .find(|c| c.id == entry.entry.category)
            .ok_or(AppError::InternalError(format!(
                "Entry's category {} does not exist.",
                entry.entry.category
            )))?,
        form_data: &form_data,
        has_unsaved_changes: false,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/entry/{entry_id}/new_previous_date")]
pub async fn new_previous_date(
    path: web::Path<(EventId, EntryId)>,
    state: web::Data<AppState>,
    data: Form<PreviousDateFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, entry_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let store = state.store.clone();
    let (entry, event, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((
            store.get_entry(&auth, entry_id)?,
            store.get_extended_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?, // TODO only get relevant category?
            auth,
        ))
    })
    .await??;

    let mut form_data = data.into_inner();
    let previous_date = form_data.validate(&rooms.iter().map(|r| r.id).collect(), &event);

    let result: util::FormSubmitResult = if let Some(mut previous_date) = previous_date {
        previous_date.previous_date.entry_id = entry_id;
        let auth_clone = auth.clone();
        web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            // TODO add explicit function to the data_store interface and use it here instead of
            //   reading + updating entry
            let entry = store.get_entry(&auth_clone, entry_id)?;
            let last_updated = entry.entry.last_updated;
            let mut entry: FullNewEntry = entry.into();
            entry.previous_dates = vec![previous_date];
            store.create_or_update_entry(&auth_clone, entry, true, Some(last_updated))?;
            Ok(())
        })
        .await?
        .into()
    } else {
        util::FormSubmitResult::ValidationError
    };

    let tmpl = NewPreviousDateFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neuer Vorheriger Termin", // TODO
            event: AnyEventData::ExtendedEvent(&event),
            current_date: Some(get_effective_date(&entry.entry.begin, &event)),
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::ByDate),
        },
        event: &event,
        entry: &entry,
        rooms: &rooms,
        rooms_by_id: rooms.iter().map(|r| (r.id, r)).collect(),
        entry_category: categories
            .iter()
            .find(|c| c.id == entry.entry.category)
            .ok_or(AppError::InternalError(format!(
                "Entry's category {} does not exist.",
                entry.entry.category
            )))?,
        form_data: &form_data,
        has_unsaved_changes: true,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Der Vorherige Termin",
        req.url_for(
            "new_previous_date_form",
            &[event_id.to_string(), entry_id.to_string()],
        )?,
        "new_previous_date_form",
        true,
        req.url_for(
            "previous_dates_overview",
            &[event_id.to_string(), entry_id.to_string()],
        )?,
        &req,
    )
}

#[derive(Deserialize, Default)]
struct PreviousDateFormData {
    previous_date_id: FormValue<Uuid>,
    day: FormValue<validation::IsoDate>,
    begin: FormValue<validation::TimeOfDay>,
    duration: FormValue<validation::NiceDurationHours>,
    rooms: FormValue<validation::CommaSeparatedUuidsFromList>,
    comment: FormValue<String>,
}

impl PreviousDateFormData {
    fn for_new_previous_date(
        previous_date_id: Uuid,
        entry: &FullEntry,
        event: &ExtendedEvent,
    ) -> Self {
        Self {
            previous_date_id: previous_date_id.into(),
            day: validation::IsoDate(get_effective_date(&entry.entry.begin, event)).into(),
            begin: validation::TimeOfDay(
                entry
                    .entry
                    .begin
                    .with_timezone(&event.timezone)
                    .naive_local()
                    .time(),
            )
            .into(),
            duration: validation::NiceDurationHours(entry.entry.end - entry.entry.begin).into(),
            rooms: validation::CommaSeparatedUuidsFromList(entry.room_ids.clone()).into(),
            comment: Default::default(),
        }
    }

    fn validate(&mut self, rooms: &Vec<Uuid>, event: &ExtendedEvent) -> Option<FullPreviousDate> {
        let previous_date_id = self.previous_date_id.validate();
        let day = self.day.validate();
        let time = self.begin.validate();
        let duration = self.duration.validate();
        let room_ids = self.rooms.validate_with(rooms);
        let comment = self.comment.validate();

        let begin =
            timestamp_from_effective_date_and_time(day?.into_inner(), time?.into_inner(), event);
        Some(FullPreviousDate {
            previous_date: PreviousDate {
                id: previous_date_id?,
                entry_id: Default::default(),
                comment: comment?,
                begin,
                end: begin + duration?.into_inner(),
            },
            room_ids: room_ids?.into_inner(),
        })
    }
}

#[derive(Template)]
#[template(path = "new_previous_date_form.html")]
struct NewPreviousDateFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    event: &'a ExtendedEvent,
    entry: &'a FullEntry,
    rooms: &'a Vec<Room>,
    rooms_by_id: BTreeMap<Uuid, &'a Room>,
    entry_category: &'a Category,
    form_data: &'a PreviousDateFormData,
    has_unsaved_changes: bool,
}

impl<'a> NewPreviousDateFormTemplate<'a> {
    fn post_url(&self) -> Result<url::Url, AppError> {
        Ok(self.base.request.url_for(
            "new_previous_date",
            &[
                self.event.basic_data.id.to_string(),
                self.entry.entry.id.to_string(),
            ],
        )?)
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
    fn day_entries(&self) -> Vec<SelectEntry<'static>> {
        event_days(&self.event.basic_data)
            .into_iter()
            .map(|date| SelectEntry {
                value: Cow::Owned(date.to_string()),
                text: Cow::Owned(format!(
                    "{} ({})",
                    date.format("%d.%m."),
                    weekday_short(&date)
                )),
            })
            .collect()
    }
}
