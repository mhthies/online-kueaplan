use actix_web::cookie::Cookie;
use actix_web::http::header::{HeaderValue, SET_COOKIE};
use actix_web::{HttpMessage, HttpRequest};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum FlashType {
    INFO,
    SUCCESS,
    WARNING,
    ERROR,
}

#[derive(Serialize, Deserialize)]
pub struct FlashMessage {
    pub flash_type: FlashType,
    pub message: String,
}

struct Flashes {
    flashes: Vec<FlashMessage>,
    dirty: bool,
}

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
    if let Some(flashes) = flashes {
        let cookie = flashes.into_cookie();
        let val = HeaderValue::from_str(&cookie.to_string())?;
        response.headers_mut().append(SET_COOKIE, val);
    }
    Ok(response)
}

// Inspiration: https://docs.rs/actix-session/latest/src/actix_session/session.rs.html
