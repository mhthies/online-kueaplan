
use diesel::prelude::*;
use uuid::Uuid;
use chrono::naive::NaiveDate;


#[derive(Queryable)]
pub struct Event {
    pub id: i32,
    pub title: String,
    pub begin_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Queryable, Insertable, AsChangeset)]
#[diesel(table_name=crate::schema::entries)]
pub struct Entry {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub responsible_person: String,
    pub is_blocker: bool,
    pub residue_of: Option<Uuid>,
    pub event_id: i32,
}

#[derive(Queryable)]
pub struct Room {
    pub id: Uuid,
    pub title: String,
    pub description: String,
}
