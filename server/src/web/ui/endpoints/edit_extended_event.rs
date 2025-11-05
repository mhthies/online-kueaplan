use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Event, EventClockInfo, ExtendedEvent};
use crate::data_store::{EventFilter, EventId, StoreError};
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::form_values::{FormValue, _FormValidSimpleValidate};
use crate::web::ui::sub_templates::form_inputs::{
    FormFieldTemplate, HiddenInputTemplate, InputType, SelectEntry, SelectTemplate,
};
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;
use serde::Deserialize;

#[get("/{event_id}/config/event/edit")]
pub async fn edit_extended_event_form(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let store = state.store.clone();
    let (event, all_events, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((
            store.get_extended_event(&auth, event_id)?,
            store.get_events(EventFilter::default())?,
            auth,
        ))
    })
    .await??;

    let form_data: ExtendedEventFormData = event.clone().into();

    let tmpl = EditExtendedEventFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Veranstaltung bearbeiten",
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::EventConfig,
        },
        event_id,
        all_events: &all_events,
        form_data: &form_data,
        has_unsaved_changes: false,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/event/edit")]
pub async fn edit_extended_event(
    path: web::Path<i32>,
    state: web::Data<AppState>,
    data: Form<ExtendedEventFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::EditEventDetails, event_id)?;
    let store = state.store.clone();
    let (old_event, all_events, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::EditEventDetails)?;
        Ok((
            store.get_extended_event(&auth, event_id)?,
            store.get_events(EventFilter::default())?,
            auth,
        ))
    })
    .await??;

    let other_event_ids = all_events
        .iter()
        .map(|event| event.id)
        .filter(|id| *id != event_id)
        .collect::<Vec<_>>();

    let mut form_data = data.into_inner();
    let event = form_data.validate(event_id, &other_event_ids);

    let result: util::FormSubmitResult = if let Some(event) = event {
        let auth_clone = auth.clone();
        web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            store.update_event(&auth_clone, event)?;
            Ok(())
        })
        .await?
        .into()
    } else {
        util::FormSubmitResult::ValidationError
    };

    let tmpl = EditExtendedEventFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Veranstaltungs-Metadaten",
            event: AnyEventData::ExtendedEvent(&old_event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::EventConfig,
        },
        event_id,
        all_events: &all_events,
        form_data: &form_data,
        has_unsaved_changes: false,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Die Veranstaltungs-Metadaten",
        req.url_for("edit_extended_event_form", &[event_id.to_string()])?,
        "edit_extended_event_form",
        false,
        req.url_for("edit_extended_event_form", &[event_id.to_string()])?,
        &req,
    )
}

#[derive(Deserialize)]
struct ExtendedEventFormData {
    title: FormValue<validation::NonEmptyString>,
    slug: FormValue<validation::MaybeEmpty<String>>,
    begin_date: FormValue<validation::IsoDate>,
    end_date: FormValue<validation::IsoDate>,
    effective_begin_of_day: FormValue<validation::TimeOfDay>,
    timezone: FormValue<validation::Timezone>,
    default_time_schedule: FormValue<validation::EventDayTimeScheduleAsJson>,
    preceding_event_id: FormValue<validation::MaybeEmpty<validation::Int32FromList>>,
    subsequent_event_id: FormValue<validation::MaybeEmpty<validation::Int32FromList>>,
}

impl ExtendedEventFormData {
    fn validate(&mut self, event_id: EventId, other_event_ids: &Vec<i32>) -> Option<ExtendedEvent> {
        let title = self.title.validate();
        let slug = self.slug.validate();
        let begin_date = self.begin_date.validate();
        let end_date = self.end_date.validate();
        let effective_begin_of_day = self.effective_begin_of_day.validate();
        let timezone = self.timezone.validate();
        let default_time_schedule = self.default_time_schedule.validate();
        let preceding_event_id = self.preceding_event_id.validate_with(other_event_ids);
        let subsequent_event_id = self.subsequent_event_id.validate_with(other_event_ids);

        let effective_begin_of_day = effective_begin_of_day?;
        let default_time_schedule = default_time_schedule?;

        if let Err(e) = default_time_schedule.0.validate(effective_begin_of_day.0) {
            self.default_time_schedule.add_error(e.into());
            return None;
        }

        Some(ExtendedEvent {
            basic_data: Event {
                id: event_id,
                title: title?.into_inner(),
                begin_date: begin_date?.into_inner(),
                end_date: end_date?.into_inner(),
                slug: slug?.0,
            },
            clock_info: EventClockInfo {
                timezone: timezone?.into_inner(),
                effective_begin_of_day: effective_begin_of_day.0,
            },
            default_time_schedule: default_time_schedule.0,
            preceding_event_id: preceding_event_id?.0.map(|v| v.into_inner()),
            subsequent_event_id: subsequent_event_id?.0.map(|v| v.into_inner()),
        })
    }
}

impl From<ExtendedEvent> for ExtendedEventFormData {
    fn from(value: ExtendedEvent) -> Self {
        Self {
            title: validation::NonEmptyString(value.basic_data.title).into(),
            slug: validation::MaybeEmpty(value.basic_data.slug).into(),
            begin_date: validation::IsoDate(value.basic_data.begin_date).into(),
            end_date: validation::IsoDate(value.basic_data.end_date).into(),
            effective_begin_of_day: validation::TimeOfDay(value.clock_info.effective_begin_of_day)
                .into(),
            timezone: validation::Timezone(value.clock_info.timezone).into(),
            default_time_schedule: validation::EventDayTimeScheduleAsJson(
                value.default_time_schedule,
            )
            .into(),
            preceding_event_id: validation::MaybeEmpty(
                value
                    .preceding_event_id
                    .map(|i| validation::Int32FromList(i)),
            )
            .into(),
            subsequent_event_id: validation::MaybeEmpty(
                value
                    .subsequent_event_id
                    .map(|i| validation::Int32FromList(i)),
            )
            .into(),
        }
    }
}

#[derive(Template)]
#[template(path = "edit_extended_event_form.html")]
struct EditExtendedEventFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event_id: EventId,
    all_events: &'a Vec<Event>,
    form_data: &'a ExtendedEventFormData,
    has_unsaved_changes: bool,
}

impl<'a> EditExtendedEventFormTemplate<'a> {
    fn post_url(&self) -> Result<url::Url, AppError> {
        Ok(self
            .base
            .request
            .url_for("edit_extended_event", &[self.event_id.to_string()])?)
    }

    fn timezone_entries() -> Vec<SelectEntry<'static>> {
        chrono_tz::TZ_VARIANTS
            .iter()
            .map(|tz| SelectEntry {
                value: tz.name().into(),
                text: tz.name().into(),
            })
            .collect()
    }

    fn other_event_entries(&self) -> Vec<SelectEntry<'a>> {
        let mut result = vec![SelectEntry {
            value: "".into(),
            text: "— keine —".into(),
        }];
        result.extend(
            self.all_events
                .iter()
                .filter(|event| event.id != self.event_id)
                .map(|event| SelectEntry {
                    value: event.id.to_string().into(),
                    text: (&event.title).into(),
                }),
        );
        result
    }
}
