use crate::data_store::auth_token::Privilege;
use crate::data_store::models::{Event, FullAnnouncement};
use crate::data_store::AnnouncementId;
use crate::web::ui::base_template::{
    BaseConfigTemplateContext, BaseTemplateContext, ConfigNavButton, MainNavButton,
};
use crate::web::ui::error::AppError;
use crate::web::ui::flash::{FlashMessage, FlashType, FlashesInterface};
use crate::web::ui::sub_templates::announcement::AnnouncementTemplate;
use crate::web::ui::util;
use crate::web::AppState;
use actix_web::web::{Html, Redirect};
use actix_web::{get, post, web, HttpRequest, Responder};
use askama::Template;

#[get("/{event_id}/config/announcements/{announcement_id}/delete")]
async fn delete_announcement_form(
    path: web::Path<(i32, AnnouncementId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, announcement_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageAnnouncements, event_id)?;
    let store = state.store.clone();
    let (announcements, event, auth) = web::block(move || -> Result<_, AppError> {
        let mut store = store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        auth.check_privilege(event_id, Privilege::ManageAnnouncements)?;
        Ok((
            store.get_announcements(&auth, event_id, None)?,
            store.get_event(event_id)?,
            auth,
        ))
    })
    .await??;

    let announcement = announcements
        .into_iter()
        .find(|a| a.announcement.id == announcement_id)
        .ok_or(AppError::EntityNotFound)?;

    let tmpl = DeleteAnnouncementTemplate {
        base: BaseTemplateContext {
            request: &req,
            page_title: "Bekanntmachung löschen",
            event: Some(&event),
            current_date: None,
            auth_token: Some(&auth),
            active_main_nav_button: Some(MainNavButton::Configuration),
        },
        base_config: BaseConfigTemplateContext {
            active_nav_button: ConfigNavButton::Announcements,
        },
        event: &event,
        announcement: &announcement,
    };

    Ok(Html::new(tmpl.render()?))
}

#[post("/{event_id}/announcement/{announcement_id}/delete")]
async fn delete_announcement(
    path: web::Path<(i32, AnnouncementId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, announcement_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageAnnouncements, event_id)?;

    let result = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        store.delete_announcement(&auth, event_id, announcement_id)?;
        Ok(())
    })
    .await?;

    match result {
        Ok(()) => {
            let notification = FlashMessage {
                flash_type: FlashType::Success,
                message: "Die Bekanntmachung wurde gelöscht.".to_string(),
                keep_open: false,
                button: None,
            };
            req.add_flash_message(notification);
            Ok(Redirect::to(
                req.url_for("manage_announcements", [&event_id.to_string()])?
                    .to_string(),
            )
            .see_other())
        }
        Err(e) => match e {
            AppError::TransactionConflict => {
                let notification = FlashMessage {
                flash_type: FlashType::Error,
                message: "Die Bekanntmachung konnte wegen eines parallelen Datenbank-Zugriff nicht gelöscht werden. Bitte erneut versuchen.".to_string(),
                keep_open: true,
                button: None,
            };
                req.add_flash_message(notification);
                Ok(Redirect::to(
                    req.url_for(
                        "delete_announcement_form",
                        &[event_id.to_string(), announcement_id.to_string()],
                    )?
                    .to_string(),
                )
                .see_other())
            }
            _ => Err(e),
        },
    }
}

#[post("/{event_id}/announcement/{announcement_id}/disable")]
async fn disable_announcement(
    path: web::Path<(i32, AnnouncementId)>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let (event_id, announcement_id) = path.into_inner();
    let session_token =
        util::extract_session_token(&state, &req, Privilege::ManageAnnouncements, event_id)?;

    let result = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        let auth = store.get_auth_token_for_session(&session_token, event_id)?;
        // TODO add explicit function to the data_store interface and use it here instead of
        //   reading + updating announcement
        let announcements = store.get_announcements(&auth, event_id, None)?;
        let mut announcement = announcements
            .into_iter()
            .find(|a| a.announcement.id == announcement_id)
            .ok_or(AppError::EntityNotFound)?;
        let last_updated = announcement.announcement.last_updated;
        announcement.announcement.show_with_days = false;
        announcement.announcement.show_with_categories = false;
        announcement.announcement.show_with_rooms = false;
        store.create_or_update_announcement(&auth, announcement.into(), Some(last_updated))?;
        Ok(())
    })
    .await?;

    match result {
        Ok(()) => {
            let notification = FlashMessage {
                flash_type: FlashType::Success,
                message: "Die Änderung wurde gespeichert.".to_string(),
                keep_open: false,
                button: None,
            };
            req.add_flash_message(notification);

            Ok(Redirect::to(
                req.url_for("manage_announcements", [&event_id.to_string()])?
                    .to_string(),
            )
            .see_other())
        }
        Err(e) => {
            let notification = match e {
                AppError::TransactionConflict => FlashMessage {
                    flash_type: FlashType::Error,
                    message: "Die Bekanntmachung konnte wegen eines parallelen Datenbank-Zugriff nicht geändert werden. Bitte erneut versuchen.".to_string(),
                    keep_open: true,
                    button: None,
                },
                AppError::ConcurrentEditConflict => FlashMessage {
                    flash_type: FlashType::Error,
                    message: "Die Bekanntmachung konnte wegen einer parallelen Änderung nicht geändert werden. Bitte erneut versuchen.".to_string(),
                    keep_open: true,
                    button: None,
                },
                _ => return Err(e),
            };

            req.add_flash_message(notification);
            Ok(Redirect::to(
                req.url_for(
                    "delete_announcement_form",
                    &[event_id.to_string(), announcement_id.to_string()],
                )?
                .to_string(),
            )
            .see_other())
        }
    }
}

#[derive(Template)]
#[template(path = "delete_announcement_form.html")]
struct DeleteAnnouncementTemplate<'a> {
    base: BaseTemplateContext<'a>,
    base_config: BaseConfigTemplateContext,
    event: &'a Event,
    announcement: &'a FullAnnouncement,
}
