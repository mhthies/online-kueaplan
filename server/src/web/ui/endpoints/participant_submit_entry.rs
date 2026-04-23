use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{
    Category, EntryState, EventClockInfo, ExtendedEvent, FullNewEntry, NewEntry, Room,
};
use crate::data_store::{DataPolicy, EntryId, EventId, StoreError};
use crate::web::time_calculation::{
    get_effective_date, most_reasonable_date, timestamp_from_effective_date_and_time,
};
use crate::web::ui::base_template::{AnyEventData, BaseTemplateContext, MainNavButton};
use crate::web::ui::error::AppError;
use crate::web::ui::form_values::{_FormValidSimpleValidate, BoolFormValue, FormValue};
use crate::web::ui::sub_templates::form_inputs::{
    CheckboxTemplate, FormFieldTemplate, HiddenInputTemplate, InputSize, InputType, SelectEntry,
    SelectTemplate,
};
use crate::web::ui::util::{event_days, weekday_short};
use crate::web::ui::{util, validation};
use crate::web::util::format_submitter_comment;
use crate::web::{time_calculation, AppState};
use actix_web::web::{Form, Html, Query};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use uuid::Uuid;

#[get("/{event_id}/submit_entry")]
async fn participant_submit_entry_form(
    path: web::Path<EventId>,
    query_data: Query<SubmitEntryQueryParams>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let date = query_data.date;
    let session_token =
        util::extract_session_token(&state, &req, Privilege::SubmitParticipantEntries, event_id)?;
    let store = state.store.clone();
    let (event, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::SubmitParticipantEntries)?;
        Ok((
            store.get_extended_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let categories = categories
        .into_iter()
        .filter(|c| !c.is_official)
        .collect::<Vec<_>>();

    let entry_id = Uuid::now_v7();
    let entry_date = date.unwrap_or_else(|| most_reasonable_date(&event));
    let category_id = categories.first().ok_or(AppError::InternalError(
        "Event does not have a single unofficial category".to_owned(),
    ))?;
    let form_data = SubmitEntryFormData::for_new_entry(
        entry_id,
        entry_date,
        category_id.id,
        event.entry_submission_mode.allows_publish_before_review(),
    );

    let tmpl = ParticipantSubmitEntryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Eintrag einreichen",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: date,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::AddEntry),
        },
        event: &event,
        form_data: &form_data,
        rooms: &rooms,
        categories: &categories,
        has_unsaved_changes: false,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/submit_entry")]
async fn participant_submit_entry(
    path: web::Path<EventId>,
    query_data: Query<SubmitEntryQueryParams>,
    data: Form<SubmitEntryFormData>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let date = query_data.date;
    let session_token =
        util::extract_session_token(&state, &req, Privilege::SubmitParticipantEntries, event_id)?;
    let store = state.store.clone();
    let (event, rooms, categories, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::SubmitParticipantEntries)?;
        Ok((
            store.get_extended_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_categories(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let categories = categories
        .into_iter()
        .filter(|c| !c.is_official)
        .collect::<Vec<_>>();

    let mut data = data.into_inner();
    let entry = data.validate(
        &rooms.iter().map(|r| r.id).collect(),
        &categories.iter().map(|c| c.id).collect(),
        None,
        &event.clock_info,
    );

    let mut entry_begin = chrono::DateTime::<chrono::Utc>::default();
    let result: util::FormSubmitResult = if let Some(mut entry) = entry {
        let auth_clone = auth.clone();
        entry.entry.event_id = event_id;
        entry_begin = entry.entry.begin;
        web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            store.submit_entry_by_participant(&auth_clone, entry)?;
            Ok(())
        })
        .await?
        .into()
    } else {
        util::FormSubmitResult::ValidationError
    };

    if let util::FormSubmitResult::PolicyViolation(violated_policy) = &result {
        match violated_policy {
            DataPolicy::EntrySubmissionNoRoomConflict => {
                data.rooms
                    .add_error("Konflikt mit anderer KüA im gleichen Raum.".to_owned());
            }
            DataPolicy::EntrySubmissionNoExclusiveConflict => {
                data.begin
                    .add_error("Konflikt mit einer exklusiven KüA.".to_owned());
            }
            _ => {}
        }
    }

    let tmpl = ParticipantSubmitEntryFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Eintrag einreichen",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: date,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::AddEntry),
        },
        event: &event,
        form_data: &data,
        rooms: &rooms,
        categories: &categories,
        has_unsaved_changes: true,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Der Eintrag",
        req.url_for("new_entry_form", &[event_id.to_string()])?,
        "edit_entry_form",
        true,
        req.url_for(
            "main_list",
            [
                &event_id.to_string(),
                &get_effective_date(&entry_begin, &event.clock_info).to_string(),
            ],
        )?,
        &req,
    )
}

/// Query parameters for the participant_submit_entry form.
#[derive(Deserialize, Serialize)]
pub struct SubmitEntryQueryParams {
    /// When given, used to pre-fill the date field of the new entry and to navigate back to this
    /// date when aborting.
    pub date: Option<chrono::NaiveDate>,
}

#[derive(Template)]
#[template(path = "participant_submit_entry_form.html")]
struct ParticipantSubmitEntryFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    event: &'a ExtendedEvent,
    form_data: &'a SubmitEntryFormData,
    categories: &'a Vec<Category>,
    rooms: &'a Vec<Room>,
    has_unsaved_changes: bool,
}

impl<'a> ParticipantSubmitEntryFormTemplate<'a> {
    fn post_url(&self) -> Result<url::Url, AppError> {
        let mut url = self.base.request.url_for(
            "participant_submit_entry",
            &[self.event.basic_data.id.to_string()],
        )?;
        url.set_query(Some(&serde_urlencoded::to_string(
            SubmitEntryQueryParams {
                date: self.base.current_date,
            },
        )?));
        Ok(url)
    }
    fn abort_url(&self) -> Result<url::Url, actix_web::error::UrlGenerationError> {
        self.base.request.url_for(
            "main_list",
            &[
                self.event.basic_data.id.to_string(),
                self.base
                    .current_date
                    .unwrap_or_else(|| time_calculation::most_reasonable_date(self.event))
                    .to_string(),
            ],
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

    fn effective_begin_of_day_milliseconds(&self) -> u64 {
        self.event
            .clock_info
            .effective_begin_of_day
            .num_seconds_from_midnight() as u64
            * 1000
            + self.event.clock_info.effective_begin_of_day.nanosecond() as u64 / 1_000_000
    }
}

#[derive(Default, Deserialize, Debug)]
struct SubmitEntryFormData {
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
    submitter_comment: FormValue<String>,
    publish_before_review: BoolFormValue,
}

impl SubmitEntryFormData {
    fn for_new_entry(
        entry_id: EntryId,
        date: chrono::NaiveDate,
        category_id: Uuid,
        publish_before_review_allowed: bool,
    ) -> Self {
        Self {
            entry_id: entry_id.into(),
            day: validation::IsoDate(date).into(),
            category: validation::UuidFromList(category_id).into(),
            duration: validation::NiceDurationHours(chrono::Duration::hours(1)).into(),
            publish_before_review: publish_before_review_allowed.into(),
            ..Self::default()
        }
    }

    fn validate(
        &mut self,
        rooms: &Vec<Uuid>,
        categories: &Vec<Uuid>,
        known_entry_id: Option<EntryId>,
        clock_info: &EventClockInfo,
    ) -> Option<FullNewEntry> {
        let entry_id = known_entry_id.or_else(|| self.entry_id.validate());
        let title = self.title.validate();
        let comment = self.comment.validate();
        let time_comment = self.time_comment.validate();
        let room_comment = self.room_comment.validate();
        let description = self.description.validate();
        let responsible_person = self.responsible_person.validate();
        let category = self.category.validate_with(categories);
        let room_ids = self.rooms.validate_with(rooms);
        let day = self.day.validate();
        let time = self.begin.validate();
        let duration = self.duration.validate();
        let submitter_comment = self.submitter_comment.validate();
        let publish_before_review = self.publish_before_review.get_value();

        let begin = timestamp_from_effective_date_and_time(
            day?.into_inner(),
            time?.into_inner(),
            clock_info,
        );
        Some(FullNewEntry {
            entry: NewEntry {
                id: entry_id?,
                title: title?.into_inner(),
                description: description?,
                responsible_person: responsible_person?,
                is_room_reservation: false,
                event_id: 0,
                begin,
                end: begin + duration?.into_inner(),
                category: category?.into_inner(),
                comment: comment?,
                time_comment: time_comment?,
                room_comment: room_comment?,
                is_exclusive: false,
                is_cancelled: false,
                state: if publish_before_review {
                    EntryState::PreliminaryPublished
                } else {
                    EntryState::SubmittedForReview
                },
                orga_comment: format_submitter_comment(&submitter_comment?),
            },
            room_ids: room_ids?.into_inner(),
            previous_dates: vec![],
        })
    }
}
