use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{FullEntry, Room};
use crate::data_store::{EntryFilter, EventId, RoomId};
use crate::web::ui::base_template::{
    BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::form_values::{FormValue, _FormValidSimpleValidate};
use crate::web::ui::sub_templates::form_inputs::{
    FormFieldTemplate, InputConfiguration, SelectEntry,
};
use crate::web::ui::time_calculation::TIME_ZONE;
use crate::web::ui::{util, validation};
use crate::web::AppState;
use actix_web::web::{Form, Html, Redirect};
use actix_web::{get, post, web, Either, HttpRequest, Responder};
use askama::Template;
use serde::Deserialize;
use std::borrow::Cow;

#[get("/{event_id}/config/rooms/{room_id}/delete")]
pub async fn delete_room_form(
    path: web::Path<(i32, RoomId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, room_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let entry_filter = EntryFilter::builder()
        .in_one_of_these_rooms(vec![room_id])
        .build();
    let (event, rooms, room_entries, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((
            store.get_event(event_id)?,
            store.get_rooms(&auth, event_id)?,
            store.get_entries_filtered(&auth, event_id, entry_filter)?,
            auth,
        ))
    })
    .await??;

    let room = rooms
        .iter()
        .find(|c| c.id == room_id)
        .ok_or(AppError::EntityNotFound)?;

    let form_data = DeleteRoomFormData::default();

    let tmpl = DeleteRoomFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Ort löschen", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        room,
        all_rooms: &rooms,
        room_entries: &room_entries,
        form_data: &form_data,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/config/rooms/{room_id}/delete")]
pub async fn delete_room(
    path: web::Path<(EventId, RoomId)>,
    state: web::Data<AppState>,
    data: Form<DeleteRoomFormData>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, room_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageCategories, event_id)?;
    let store = state.store.clone();
    let (event, rooms, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((
            store.get_event(event_id)?,
            store.get_rooms(&auth, event_id)?,
            auth,
        ))
    })
    .await??;

    let mut form_data = data.into_inner();
    let validated_data = form_data.validate(
        &rooms
            .iter()
            .filter(|c| c.id != room_id)
            .map(|c| c.id)
            .collect::<Vec<RoomId>>(),
    );

    let result = if let Some(replacement_data) = validated_data {
        let store = state.store.clone();
        let auth = auth.clone();
        Some(
            web::block(move || -> Result<_, AppError> {
                let mut store = store.get_facade()?;
                store.delete_room(
                    &auth,
                    event_id,
                    room_id,
                    &replacement_data.replace_rooms,
                    &replacement_data.add_room_comment,
                )?;
                Ok(())
            })
            .await?,
        )
    } else {
        None
    };

    match result {
        Some(Ok(())) => {
            let notification = FlashMessage {
                flash_type: FlashType::Success,
                message: "Der Ort wurde gelöscht.".to_string(),
                keep_open: false,
                button: None,
            };
            req.add_flash_message(notification);
            return Ok(Either::Left(
                Redirect::to(
                    req.url_for("manage_rooms", [&event_id.to_string()])?
                        .to_string(),
                )
                .see_other(),
            ));
        }
        None => {
            let notification = FlashMessage {
                flash_type: FlashType::Error,
                message: "Eingegebene Daten sind ungültig. Bitte markierte Felder überprüfen."
                    .to_owned(),
                keep_open: false,
                button: None,
            };
            req.add_flash_message(notification);
        }
        Some(Err(e)) => match e {
            AppError::TransactionConflict => {
                let notification = FlashMessage {
                    flash_type: FlashType::Error,
                    message: "Der Ort konnte wegen eines parallelen Datenbank-Zugriff nicht gelöscht werden. Bitte erneut versuchen.".to_string(),
                    keep_open: true,
                    button: None,
                };
                req.add_flash_message(notification);
            }
            _ => {
                return Err(e);
            }
        },
    };

    // TODO deduplicate code with delete_room_form
    let entry_filter = EntryFilter::builder()
        .in_one_of_these_rooms(vec![room_id])
        .build();
    let store = state.store.clone();
    let (mut room_entries, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        auth.check_privilege(event_id, Privilege::ManageCategories)?;
        Ok((
            store.get_entries_filtered(&auth, event_id, entry_filter)?,
            auth,
        ))
    })
    .await??;

    let room = rooms
        .iter()
        .find(|c| c.id == room_id)
        .ok_or(AppError::EntityNotFound)?;
    room_entries.sort_by_key(|e| e.entry.begin);

    let tmpl = DeleteRoomFormTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Ort löschen", // TODO
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Categories,
        },
        event_id,
        room,
        all_rooms: &rooms,
        room_entries: &room_entries,
        form_data: &form_data,
    };

    Ok(Either::Right(Html::new(tmpl.render()?)))
}

#[derive(Deserialize, Default)]
struct DeleteRoomFormData {
    replace_rooms: FormValue<validation::CommaSeparatedUuidsFromList>,
    add_room_comment: FormValue<String>,
}

impl DeleteRoomFormData {
    fn validate(&mut self, rooms: &Vec<RoomId>) -> Option<DeleteRoomReplacementData> {
        let replace_rooms = self.replace_rooms.validate_with(rooms);
        let add_room_comment = self.add_room_comment.validate();

        Some(DeleteRoomReplacementData {
            replace_rooms: replace_rooms?.into_inner(),
            add_room_comment: add_room_comment?,
        })
    }
}

struct DeleteRoomReplacementData {
    replace_rooms: Vec<RoomId>,
    add_room_comment: String,
}

#[derive(Template)]
#[template(path = "delete_room_form.html")]
struct DeleteRoomFormTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event_id: EventId,
    room: &'a Room,
    all_rooms: &'a Vec<Room>,
    room_entries: &'a Vec<FullEntry>,
    form_data: &'a DeleteRoomFormData,
}

impl DeleteRoomFormTemplate<'_> {
    fn other_room_entries(&self) -> Vec<SelectEntry> {
        self.all_rooms
            .iter()
            .filter(|c| c.id != self.room.id)
            .map(|c| SelectEntry {
                value: Cow::Owned(c.id.to_string()),
                text: Cow::Borrowed(c.title.as_str()),
            })
            .collect()
    }

    fn post_url(&self) -> Result<url::Url, AppError> {
        Ok(self.base.request.url_for(
            "delete_room",
            [&self.event_id.to_string(), &self.room.id.to_string()],
        )?)
    }

    fn to_our_timezone(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
        timestamp.with_timezone(&TIME_ZONE).naive_local()
    }
}
