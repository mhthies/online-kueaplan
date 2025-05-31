use crate::data_store::models::Announcement;
use crate::web::ui::util::{
    announcement_type_color, announcement_type_icon, announcement_type_name,
};
use askama::Template;

#[derive(Template)]
#[template(path = "sub_templates/announcement.html")]
pub struct AnnouncementTemplate<'a> {
    announcement: &'a Announcement,
}

impl<'a> AnnouncementTemplate<'a> {
    pub fn new(announcement: &'a Announcement) -> Self {
        Self { announcement }
    }
}

mod filters {
    pub use crate::web::ui::askama_filters::markdown;
}
