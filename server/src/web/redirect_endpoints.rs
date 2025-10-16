use crate::web::ui::error::AppError;
use crate::web::AppState;
use actix_web::web::Redirect;
use actix_web::{get, web, HttpRequest, Responder};

#[get("/{event_slug}")]
async fn event_redirect_by_slug(
    path: web::Path<String>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<impl Responder, AppError> {
    let event_slug = path.into_inner();
    let result = web::block(move || -> Result<_, AppError> {
        let mut store = state.store.get_facade()?;
        Ok(store.get_event_by_slug(&event_slug)?)
    })
    .await?;

    // Non-existing event slugs should be reported and logged like a non-existent URL
    if let Err(AppError::EntityNotFound) = result {
        return Err(AppError::PageNotFound);
    }
    let event = result?;

    Ok(Redirect::to(
        req.url_for("event_index", &[event.id.to_string()])?
            .to_string(),
    )
    .see_other())
}
