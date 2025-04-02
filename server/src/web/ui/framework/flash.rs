//! This module provides functionality to "flash" notification messages to the user, i.e. show them
//! once on the next loaded page after their creation.
//! The important part is that this has to work across requests, to allow creating flash messages
//! upon any request, even if the request results in a redirect to another page.
//! This is commonly used for processing POST requests resulting from HTML form submits:
//! The submitted data is processed, a "success" flash message is created and the user is redirected
//! to another page.
//! On this page (whatever it is), the flash message can be displayed.
//!
//! This is achieved by storing the pending flash messages in a cookie until they are displayed.
//! As soon as the messages have been displayed, the cooke is cleared.
//!
//! For simple usage in the actix-web application, this module stores the list of pending flash
//! messages in an extension of the [actix_web::HttpRequest] object.
//! Adding a new message as well as retrieving and clearing all pending flash messages is possible
//! through the [FlashesInterface] trait, on a (non-mutable) reference to the HttpRequest.
//!
//! In addition, the web application needs to be wrapped in the provided middleware function
//! [flash_middleware] for reading the list of pending flash messages from the cookie into th
//! HttpRequest extension and vice versa.
//!
//! Typical usage:
//! ```ignore
//! use flash::{FlashMessage, FlashType, FlashesInterface, flash_middleware};
//!
//! #[actix_web::get("/item")]
//! async fn show_item(req: actix_web::HttpRequest) -> impl actix_web::Responder {
//!     let flashes = req.get_and_clear_flashes();
//!     Ok(actix_web::web::Html::new(format!(
//!         "<div class=\"notifications´\">{:?}</div><h1>The Item</h1>...", flashes)))
//! }
//!
//! #[actix_web::post("/item")]
//! async fn modify_item(req: actix_web::HttpRequest) -> impl actix_web::Responder {
//!     req.add_flash_message(FlashMessage {
//!         flash_type: FlashType::SUCCESS,
//!         message: "Item has been modified.".to_owned(),
//!     });
//!     actix_web::web::Redirect::to(req.url_for("show_item", &[]).unwrap().to_string()).see_other()
//! }
//!
//! let app = actix_web::App::new()
//!         .service(show_item)
//!         .service(modify_item)
//!         .wrap(actix_web::middleware::from_fn(flash_middleware));
//! ```
use actix_web::cookie::Cookie;
use actix_web::http::header::{HeaderValue, SET_COOKIE};
use actix_web::{post, HttpMessage, HttpRequest};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum FlashType {
    INFO,
    SUCCESS,
    WARNING,
    ERROR,
}

/// A single notification message to be flashed to the user
#[derive(Serialize, Deserialize)]
pub struct FlashMessage {
    pub flash_type: FlashType,
    pub message: String,
}

/// Struct for holding the pending flash messages, to be added as an extension to a
/// [actix_web::HttpRequest] object.
struct Flashes {
    flashes: Vec<FlashMessage>,
    /// If true, the list of flash messages has been changed during processing the current request,
    /// such that deviates from the list of messages in the client's cookie.
    dirty: bool,
}

/// The name of the HTTP cookie used for storing the pending flash messages
const COOKIE_NAME: &str = "flash";

impl Flashes {
    fn from_cookie(request: &HttpRequest) -> Result<Self, Box<dyn std::error::Error>> {
        let cookie = request.cookie(COOKIE_NAME);
        if let Some(cookie) = cookie {
            Ok(Flashes {
                flashes: serde_json::from_str(cookie.value())?,
                dirty: false,
            })
        } else {
            Ok(Flashes {
                flashes: vec![],
                dirty: false,
            })
        }
    }

    fn into_cookie(self) -> Cookie<'static> {
        let mut result = Cookie::new(
            COOKIE_NAME,
            serde_json::to_string(&self.flashes).expect("Flashes should be serializable as JSON"),
        );
        result.set_path("/");
        result
    }
}

pub trait FlashesInterface {
    fn add_flash_message(&self, flash: FlashMessage);

    fn get_and_clear_flashes(&self) -> Vec<FlashMessage>;
}

impl FlashesInterface for HttpRequest {
    fn add_flash_message(&self, flash: FlashMessage) {
        if let Some(flashes) = self.extensions_mut().get_mut::<Flashes>() {
            flashes.flashes.push(flash);
            flashes.dirty = true;
            return;
        }
        // Must not be within the `match` statement to avoid panicking of the `extensions` RefCell
        self.extensions_mut().insert(Flashes {
            flashes: vec![flash],
            dirty: true,
        });
    }

    fn get_and_clear_flashes(&self) -> Vec<FlashMessage> {
        self.extensions_mut()
            .get_mut::<Flashes>()
            .map(|flashes| {
                flashes.dirty = true;
                std::mem::take(&mut flashes.flashes)
            })
            .unwrap_or(Vec::new())
    }
}

pub async fn flash_middleware(
    req: actix_web::dev::ServiceRequest,
    next: actix_web::middleware::Next<impl actix_web::body::MessageBody>,
) -> Result<actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>, actix_web::Error> {
    let flashes = Flashes::from_cookie(req.request());
    // Ignore errors while parsing flashes from Request
    if flashes.is_ok() {
        req.extensions_mut().insert(flashes.unwrap());
    }

    let mut response = next.call(req).await?;

    let flashes = response.request().extensions_mut().remove::<Flashes>();
    // TODO only clear flashes when response was not an Err
    if let Some(flashes) = flashes {
        let cookie = flashes.into_cookie();
        let val = HeaderValue::from_str(&cookie.to_string())?;
        response.headers_mut().append(SET_COOKIE, val);
    }
    Ok(response)
}

// Inspiration: https://docs.rs/actix-session/latest/src/actix_session/session.rs.html
