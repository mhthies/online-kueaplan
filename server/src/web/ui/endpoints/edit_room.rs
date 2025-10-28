use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{NewRoom, Room};
use crate::data_store::{EventId, RoomId, StoreError};
use crate::web::ui::base_template::{
    AnyEventData, BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::form_values::{FormValue, _FormValidSimpleValidate};
use crate::web::ui::sub_templates::form_inputs::{
    FormFieldTemplate, HiddenInputTemplate, InputConfiguration, InputSize, InputType,
};
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;
use serde::Deserialize;
use uuid::Uuid;

#[get("/{event_id}/config/rooms/{room_id}/edit")]
pub async fn edit_room_form(
    path: web::Path<(i32, RoomId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, room_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageRooms, event_id)?;
    let store = state.store.clone();
    let (event, rooms, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageRooms)?;
        Ok((
            // TODO only get required room
            store.get_extended_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let room = rooms
        .into_iter()
        .find(|c| c.id == room_id)
        .ok_or(AppError::EntityNotFound)?;
    let form_data: RoomFormData = room.into();

    let tmpl = EditRoomFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Ort bearbeiten", // TODO
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Rooms,
        },
        event_id,
        form_data: &form_data,
        room_id: Some(&room_id),
        has_unsaved_changes: false,
        is_new_room: false,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/rooms/{room_id}/edit")]
pub async fn edit_room(
    path: web::Path<(EventId, RoomId)>,
    state: web::Data<AppState>,
    data: Form<RoomFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, room_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageRooms, event_id)?;
    let store = state.store.clone();
    let (event, rooms, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageRooms)?;
        Ok((
            // TODO only get required room
            store.get_extended_event(&auth, event_id)?,
            store.get_rooms(&auth, event_id)?,
            auth,
        ))
    })
    .await??;
    let _old_room = rooms
        .into_iter()
        .find(|c| c.id == room_id)
        .ok_or(AppError::EntityNotFound)?;

    let mut form_data = data.into_inner();
    let room = form_data.validate(Some(room_id));

    let result: util::FormSubmitResult = if let Some(mut room) = room {
        room.event_id = event_id;
        let auth_clone = auth.clone();
        web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            store.create_or_update_room(&auth_clone, room)?;
            Ok(())
        })
        .await?
        .into()
    } else {
        util::FormSubmitResult::ValidationError
    };

    let tmpl = EditRoomFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Ort bearbeiten", // TODO
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Rooms,
        },
        event_id,
        form_data: &form_data,
        room_id: Some(&room_id),
        has_unsaved_changes: false,
        is_new_room: false,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Der Ort",
        req.url_for(
            "edit_room_form",
            &[event_id.to_string(), room_id.to_string()],
        )?,
        "edit_room_form",
        false,
        req.url_for("manage_rooms", &[event_id.to_string()])?,
        &req,
    )
}

#[get("/{event_id}/config/rooms/new")]
pub async fn new_room_form(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageRooms, event_id)?;
    let store = state.store.clone();
    let (event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageRooms)?;
        Ok((store.get_extended_event(&auth, event_id)?, auth))
    })
    .await??;

    let room_id = Uuid::now_v7();
    let form_data: RoomFormData = RoomFormData::for_new_room(room_id);

    let tmpl = EditRoomFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neuer Ort", // TODO
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Rooms,
        },
        event_id,
        form_data: &form_data,
        room_id: None,
        has_unsaved_changes: false,
        is_new_room: true,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/rooms/new")]
pub async fn new_room(
    path: web::Path<EventId>,
    state: web::Data<AppState>,
    data: Form<RoomFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_id = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageRooms, event_id)?;
    let store = state.store.clone();
    let (event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageRooms)?;
        Ok((store.get_extended_event(&auth, event_id)?, auth))
    })
    .await??;

    let mut form_data = data.into_inner();
    let room = form_data.validate(None);

    let result: util::FormSubmitResult = if let Some(mut room) = room {
        room.event_id = event_id;
        let auth_clone = auth.clone();
        web::block(move || -> Result<_, StoreError> {
            let mut store = state.store.get_facade()?;
            store.create_or_update_room(&auth_clone, room)?;
            Ok(())
        })
        .await?
        .into()
    } else {
        util::FormSubmitResult::ValidationError
    };

    let tmpl = EditRoomFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Neuer Ort", // TODO
            event: AnyEventData::ExtendedEvent(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Rooms,
        },
        event_id,
        form_data: &form_data,
        room_id: None,
        has_unsaved_changes: true,
        is_new_room: true,
    };

    util::create_edit_form_response(
        result,
        &tmpl,
        "Der Ort",
        req.url_for("new_room_form", &[event_id.to_string()])?,
        "edit_room_form",
        true,
        req.url_for("manage_rooms", &[event_id.to_string()])?,
        &req,
    )
}

#[derive(Deserialize, Default)]
struct RoomFormData {
    /// Id of the room, only used for creating new rooms (for editing existing entries, the
    /// id is taken from the URL and passed to [validate] as `known_id` instead)
    room_id: FormValue<Uuid>,
    title: FormValue<validation::NonEmptyString>,
    description: FormValue<String>,
}

impl RoomFormData {
    fn for_new_room(room_id: RoomId) -> Self {
        Self {
            room_id: room_id.into(),
            ..Self::default()
        }
    }

    fn validate(&mut self, known_id: Option<RoomId>) -> Option<NewRoom> {
        let room_id = known_id.or_else(|| self.room_id.validate());
        let title = self.title.validate();
        let description = self.description.validate();

        Some(NewRoom {
            id: room_id?,
            title: title?.into_inner(),
            description: description?,
            event_id: 0,
        })
    }
}

impl From<Room> for RoomFormData {
    fn from(value: Room) -> Self {
        Self {
            room_id: value.id.into(),
            title: validation::NonEmptyString(value.title).into(),
            description: value.description.into(),
        }
    }
}

#[derive(Template)]
#[template(path = "edit_room_form.html")]
struct EditRoomFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event_id: EventId,
    form_data: &'a RoomFormData,
    room_id: Option<&'a RoomId>,
    has_unsaved_changes: bool,
    is_new_room: bool,
}

impl EditRoomFormTemplate<'_> {
    fn post_url(&self) -> Result<url::Url, AppError> {
        if self.is_new_room {
            Ok(self
                .base
                .request
                .url_for("new_room", &[self.event_id.to_string()])?)
        } else {
            Ok(self.base.request.url_for(
                "edit_room",
                &[
                    self.event_id.to_string(),
                    self.room_id
                        .expect("For non-new entries, `room_id` should always be known.")
                        .to_string(),
                ],
            )?)
        }
    }
}
