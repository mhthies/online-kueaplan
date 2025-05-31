use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{
    AnnouncementType, Category, Event, FullAnnouncement, FullNewAnnouncement, NewAnnouncement, Room,
};
use crate::data_store::{AnnouncementId, CategoryId, EventId, RoomId, StoreError};
use crate::web::ui::base_template::{
    BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::form_values::{
    BoolFormValue, FormValue, FormValueRepresentation, ValidateFromFormInput,
    _FormValidSimpleValidate,
};
use crate::web::ui::sub_templates::form_inputs::{
    CheckboxTemplate, FormFieldTemplate, HiddenInputTemplate, InputConfiguration, InputType,
    SelectEntry, SelectTemplate,
};
use crate::web::ui::util::{announcement_type_name, event_days};
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;
use serde::Deserialize;
use std::borrow::Cow;
use uuid::Uuid;

#[get("/{event_id}/config/announcements/{announcement_id}/edit")]
pub async fn edit_announcement_form(
    path: web::Path<(i32, AnnouncementId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, announcement_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let store = state.store.clone();
    let (event, announcements, categories, rooms, auth) =
        web::block(move || -> Result<_, AppError> {
            let mut store = store.get_facade()?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            auth.check_privilege(event_id, Privilege::ManageCategories)?;
            Ok((
                // TODO only get required announcement
                store.get_event(event_id)?,
                store.get_announcements(&auth, event_id, None)?,
                store.get_categories(&auth, event_id)?,
                store.get_rooms(&auth, event_id)?,
                auth,
            ))
        })
        .await??;

    let announcement = announcements
        .into_iter()
        .filter(|a| a.announcement.id == announcement_id)
        .next()
        .ok_or(AppError::EntityNotFound)?;
    let form_data: AnnouncementFormData = announcement.into();

    let tmpl = EditAnnouncementFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Bekanntmachung bearbeiten", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        form_data: &form_data,
        announcement_id: Some(&announcement_id),
        has_unsaved_changes: false,
        is_new_announcement: false,
        event: &event,
        categories: &categories,
        rooms: &rooms,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/announcements/{announcement_id}/edit")]
pub async fn edit_announcement(
    path: web::Path<(EventId, AnnouncementId)>,
    state: web::Data<AppState>,
    data: Form<AnnouncementFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, announcement_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let store = state.store.clone();
    let (event, announcements, categories, rooms, auth) =
        web::block(move || -> Result<_, AppError> {
            let mut store = store.get_facade()?;
            let auth = store.get_auth_token_for_session(&session_token, event_id)?;
            auth.check_privilege(event_id, Privilege::ManageCategories)?;
            Ok((
                // TODO only get required announcement
                store.get_event(event_id)?,
                store.get_announcements(&auth, event_id, None)?,
                store.get_categories(&auth, event_id)?,
                store.get_rooms(&auth, event_id)?,
                auth,
            ))
        })
        .await??;
    let _old_announcement = announcements
        .into_iter()
        .filter(|a| a.announcement.id == announcement_id)
        .next()
        .ok_or(AppError::EntityNotFound)?;

    let mut form_data = data.into_inner();
    let announcement = form_data.validate(
        Some(announcement_id),
        &categories.iter().map(|c| c.id).collect(),
        &rooms.iter().map(|r| r.id).collect(),
    );

    let result: util::FormSubmitResult =
        if let Some((mut announcement, previous_last_updated)) = announcement {
            announcement.announcement.event_id = event_id;
            let auth_clone = auth.clone();
            web::block(move || -> Result<_, StoreError> {
                let mut store = state.store.get_facade()?;
                store.create_or_update_announcement(
                    &auth_clone,
                    announcement,
                    previous_last_updated,
                )?;
                Ok(())
            })
            .await?
            .into()
        } else {
            util::FormSubmitResult::ValidationError
        };

    let tmpl = EditAnnouncementFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Bekanntmachung bearbeiten", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        form_data: &form_data,
        announcement_id: Some(&announcement_id),
        has_unsaved_changes: false,
        is_new_announcement: false,
        event: &event,
        categories: &categories,
        rooms: &rooms,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Die Bekanntmachung",
        req.url_for(
            "edit_announcement_form",
            &[event_id.to_string(), announcement_id.to_string()],
        )?,
        "edit_announcement_form",
        false,
        req.url_for("manage_announcements", &[event_id.to_string()])?,
        &req,
    )
}

#[get("/{event_id}/config/announcements/new")]
pub async fn new_announcement_form(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let store = state.store.clone();
    let (event, categories, rooms, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((
            store.get_event(event_id)?,
            store.get_categories(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let announcement_id = Uuid::now_v7();
    let form_data: AnnouncementFormData =
        AnnouncementFormData::for_new_announcement(announcement_id);

    let tmpl = EditAnnouncementFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neue Bekanntmachung", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        form_data: &form_data,
        announcement_id: None,
        has_unsaved_changes: false,
        is_new_announcement: true,
        event: &event,
        categories: &categories,
        rooms: &rooms,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/announcements/new")]
pub async fn new_announcement(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    data: Form<AnnouncementFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let store = state.store.clone();
    let (event, categories, rooms, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((
            store.get_event(event_id)?,
            store.get_categories(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let mut form_data = data.into_inner();
    let announcement = form_data.validate(
        None,
        &categories.iter().map(|c| c.id).collect(),
        &rooms.iter().map(|r| r.id).collect(),
    );

    let result: util::FormSubmitResult =
        if let Some((mut announcement, _previous_last_updated)) = announcement {
            announcement.announcement.event_id = event_id;
            let auth_clone = auth.clone();
            web::block(move || -> Result<_, StoreError> {
                let mut store = state.store.get_facade()?;
                store.create_or_update_announcement(&auth_clone, announcement, None)?;
                Ok(())
            })
            .await?
            .into()
        } else {
            util::FormSubmitResult::ValidationError
        };

    let tmpl = EditAnnouncementFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neuer Ort", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        form_data: &form_data,
        announcement_id: None,
        has_unsaved_changes: true,
        is_new_announcement: true,
        event: &event,
        categories: &categories,
        rooms: &rooms,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Die Bekanntmachung",
        req.url_for("new_announcement_form", &[event_id.to_string()])?,
        "edit_announcement_form",
        true,
        req.url_for("manage_announcements", &[event_id.to_string()])?,
        &req,
    )
}

#[derive(Debug)]
struct AnnouncementTypeValue(AnnouncementType);

impl Default for AnnouncementTypeValue {
    fn default() -> Self {
        Self(AnnouncementType::INFO)
    }
}

impl FormValueRepresentation for AnnouncementTypeValue {
    fn into_form_value_string(self) -> String {
        let value: i32 = self.0.into();
        value.to_string()
    }
}
impl ValidateFromFormInput for AnnouncementTypeValue {
    fn from_form_value(value: &str) -> Result<Self, String> {
        let v = value
            .parse::<i32>()
            .map_err(|e| format!("Keine Zahl: {}", e))?;
        Ok(Self(v.try_into().map_err(|_| {
            "Kein gültiger Bekanntmachungs-Typ".to_string()
        })?))
    }
}

#[derive(Deserialize, Default)]
struct AnnouncementFormData {
    /// Id of the announcement, only used for creating new announcements (for editing existing
    /// announcements, the id is taken from the URL and passed to [validate] as `known_id` instead)
    announcement_id: FormValue<Uuid>,
    announcement_type: FormValue<AnnouncementTypeValue>,
    text: FormValue<String>,
    show_with_days: BoolFormValue,
    begin_date: FormValue<validation::MaybeEmpty<validation::IsoDate>>,
    end_date: FormValue<validation::MaybeEmpty<validation::IsoDate>>,
    show_with_categories: BoolFormValue,
    categories: FormValue<validation::CommaSeparatedUuidsFromList>,
    show_with_rooms: BoolFormValue,
    rooms: FormValue<validation::CommaSeparatedUuidsFromList>,
    sort_key: FormValue<validation::Int32>,
    /// `last_updated` value of the (original) announcement. Used for detecting editing conflicts.
    /// Only used for editing existing announcements; can be empty/missing when creating new
    /// announcements.
    last_updated: FormValue<validation::SimpleTimestampMicroseconds>,
}

impl AnnouncementFormData {
    fn for_new_announcement(announcement_id: AnnouncementId) -> Self {
        Self {
            announcement_id: announcement_id.into(),
            ..Self::default()
        }
    }

    fn validate(
        &mut self,
        known_id: Option<AnnouncementId>,
        category_ids: &Vec<CategoryId>,
        room_ids: &Vec<RoomId>,
    ) -> Option<(FullNewAnnouncement, Option<chrono::DateTime<chrono::Utc>>)> {
        let announcement_id = known_id.or_else(|| self.announcement_id.validate());
        let announcement_type = self.announcement_type.validate();
        let text = self.text.validate();
        let begin_date = self.begin_date.validate();
        let end_date = self.end_date.validate();
        let categories = self.categories.validate_with(category_ids);
        let rooms = self.rooms.validate_with(room_ids);
        let sort_key = self.sort_key.validate();
        let previous_last_updated = self.last_updated.validate();

        let begin_date = begin_date?;
        let end_date = end_date?;
        if let Some(ref begin_date) = begin_date.0 {
            if let Some(ref end_date) = end_date.0 {
                if end_date.0 < begin_date.0 {
                    self.end_date
                        .add_error("Darf nicht vor dem Start-Datum liegen.".to_owned());
                    return None;
                }
            }
        }
        let rooms = rooms?.0;
        let categories = categories?.0;

        Some((
            FullNewAnnouncement {
                announcement: NewAnnouncement {
                    id: announcement_id?,
                    event_id: 0,
                    announcement_type: announcement_type?.0,
                    text: text?,
                    show_with_days: self.show_with_days.get_value(),
                    begin_date: begin_date.0.map(|v| v.0),
                    end_date: end_date.0.map(|v| v.0),
                    show_with_categories: self.show_with_categories.get_value(),
                    show_with_all_categories: categories.is_empty(),
                    show_with_rooms: self.show_with_rooms.get_value(),
                    show_with_all_rooms: rooms.is_empty(),
                    sort_key: sort_key?.0,
                },
                category_ids: categories,
                room_ids: rooms,
            },
            previous_last_updated.map(|v| v.0),
        ))
    }
}

impl From<FullAnnouncement> for AnnouncementFormData {
    fn from(value: FullAnnouncement) -> Self {
        Self {
            announcement_id: value.announcement.id.into(),
            announcement_type: AnnouncementTypeValue(value.announcement.announcement_type).into(),
            text: value.announcement.text.into(),
            show_with_days: value.announcement.show_with_days.into(),
            begin_date: validation::MaybeEmpty(
                value
                    .announcement
                    .begin_date
                    .map(|v| validation::IsoDate(v)),
            )
            .into(),
            end_date: validation::MaybeEmpty(
                value.announcement.end_date.map(|v| validation::IsoDate(v)),
            )
            .into(),
            show_with_categories: value.announcement.show_with_categories.into(),
            categories: validation::CommaSeparatedUuidsFromList(value.category_ids).into(),
            show_with_rooms: value.announcement.show_with_rooms.into(),
            rooms: validation::CommaSeparatedUuidsFromList(value.room_ids).into(),
            sort_key: validation::Int32(value.announcement.sort_key).into(),
            last_updated: validation::SimpleTimestampMicroseconds(value.announcement.last_updated)
                .into(),
        }
    }
}

#[derive(Template)]
#[template(path = "edit_announcement_form.html")]
struct EditAnnouncementFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event_id: EventId,
    form_data: &'a AnnouncementFormData,
    announcement_id: Option<&'a AnnouncementId>,
    has_unsaved_changes: bool,
    is_new_announcement: bool,
    event: &'a Event,
    categories: &'a Vec<Category>,
    rooms: &'a Vec<Room>,
}

impl<'a> EditAnnouncementFormTemplate<'a> {
    fn post_url(&self) -> Result<url::Url, AppError> {
        if self.is_new_announcement {
            Ok(self
                .base
                .request
                .url_for("new_announcement", &[self.event_id.to_string()])?)
        } else {
            Ok(self.base.request.url_for(
                "edit_announcement",
                &[
                    self.event_id.to_string(),
                    self.announcement_id
                        .expect(
                            "For non-new announcements, `announcement_id` should always be known.",
                        )
                        .to_string(),
                ],
            )?)
        }
    }

    fn announcement_type_entries() -> Vec<SelectEntry<'static>> {
        [AnnouncementType::INFO, AnnouncementType::WARNING]
            .iter()
            .map(|t| SelectEntry {
                value: Cow::Owned(i32::from(t.clone()).to_string()),
                text: Cow::Borrowed(announcement_type_name(*t)),
            })
            .collect()
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
    fn begin_date_entries(&self) -> Vec<SelectEntry<'static>> {
        let days = event_days(self.event);
        let mut result = vec![SelectEntry {
            value: Cow::Borrowed(""),
            text: Cow::Borrowed("Anfang"),
        }];
        result.extend(days.into_iter().skip(1).map(|date| SelectEntry {
            value: Cow::Owned(date.to_string()),
            text: Cow::Owned(date.format("%d.%m.").to_string()),
        }));
        result
    }
    fn end_date_entries(&self) -> Vec<SelectEntry<'static>> {
        let days = event_days(self.event);
        let num = days.len();
        let mut result: Vec<_> = days
            .into_iter()
            .take(num - 1)
            .map(|date| SelectEntry {
                value: Cow::Owned(date.to_string()),
                text: Cow::Owned(date.format("%d.%m.").to_string()),
            })
            .collect();
        result.push(SelectEntry {
            value: Cow::Borrowed(""),
            text: Cow::Borrowed("Ende"),
        });
        result
    }
}
