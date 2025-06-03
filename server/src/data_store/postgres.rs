use super::{
    models, schema, AnnouncementFilter, AnnouncementId, CategoryId, EntryFilter, EntryId, EventId,
    KuaPlanStore, KueaPlanStoreFacade, RoomId, StoreError,
};
use crate::auth_session::SessionToken;
use crate::data_store::auth_token::{
    AccessRole, AuthToken, EnumMemberNotExistingError, GlobalAuthToken, Privilege,
};
use diesel::dsl::exists;
use diesel::expression::AsExpression;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::PooledConnection;
use uuid::Uuid;

#[derive(Clone)]
pub struct PgDataStore {
    pool: diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>>,
}

impl PgDataStore {
    pub fn new(database_url: &str) -> Result<Self, StoreError> {
        let connection_manager = diesel::r2d2::ConnectionManager::<PgConnection>::new(database_url);
        Ok(Self {
            pool: diesel::r2d2::Pool::builder()
                .test_on_check_out(true)
                .min_idle(Some(2))
                .build(connection_manager)?,
        })
    }
}

impl KuaPlanStore for PgDataStore {
    fn get_facade<'a>(&'a self) -> Result<Box<dyn KueaPlanStoreFacade + 'a>, StoreError> {
        Ok(Box::new(PgDataStoreFacade::with_pooled_connection(
            self.pool.get()?,
        )))
    }
}

pub struct PgDataStoreFacade {
    connection: diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>,
}

impl PgDataStoreFacade {
    pub fn with_pooled_connection(
        connection: diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>,
    ) -> Self {
        Self { connection }
    }
}

/// Create an Sql expression to check if a row has been created or updated by a Postgres "upsert"
/// statement
fn sql_upsert_is_updated() -> diesel::expression::SqlLiteral<diesel::sql_types::Bool> {
    // See https://stackoverflow.com/q/34762732 and https://stackoverflow.com/q/49597793
    diesel::dsl::sql("xmax::text <> '0'")
}

impl KueaPlanStoreFacade for PgDataStoreFacade {
    fn get_event(&mut self, event_id: i32) -> Result<models::Event, StoreError> {
        use schema::events::dsl::*;

        events
            .filter(id.eq(event_id))
            .select(models::Event::as_select())
            .first::<models::Event>(&mut self.connection)
            .map_err(|e| e.into())
    }

    fn get_extended_event(
        &mut self,
        _auth_token: &AuthToken,
        event_id: i32,
    ) -> Result<models::ExtendedEvent, StoreError> {
        use schema::events::dsl::*;

        events
            .filter(id.eq(event_id))
            .select(models::ExtendedEvent::as_select())
            .first::<models::ExtendedEvent>(&mut self.connection)
            .map_err(|e| e.into())
    }

    fn create_event(
        &mut self,
        auth_token: &GlobalAuthToken,
        event: models::NewEvent,
    ) -> Result<i32, StoreError> {
        use schema::events::dsl::*;
        auth_token.check_privilege(Privilege::CreateEvents)?;

        Ok(diesel::insert_into(events)
            .values(&event)
            .get_results::<models::Event>(&mut self.connection)
            .map(|e| e[0].id)?)
    }

    fn get_entries_filtered(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: i32,
        filter: EntryFilter,
    ) -> Result<Vec<models::FullEntry>, StoreError> {
        use diesel::dsl::not;
        use schema::entries::dsl::*;
        auth_token.check_privilege(the_event_id, Privilege::ShowKueaPlan)?;

        self.connection.transaction(|connection| {
            let the_entries = entries
                .filter(event_id.eq(the_event_id))
                .filter(not(deleted))
                .filter(entry_filter_to_sql(filter))
                .order_by((begin.asc(), end.asc()))
                .select(models::Entry::as_select())
                .load::<models::Entry>(connection)?;

            let the_entry_rooms = models::EntryRoomMapping::belonging_to(&the_entries)
                .inner_join(schema::rooms::table)
                .filter(not(schema::rooms::deleted))
                .select(models::EntryRoomMapping::as_select())
                .load::<models::EntryRoomMapping>(connection)?
                .grouped_by(&the_entries);

            let the_previous_dates = models::PreviousDate::belonging_to(&the_entries)
                .select(models::PreviousDate::as_select())
                .load::<models::PreviousDate>(connection)?;

            let the_previous_date_rooms =
                models::PreviousDateRoomMapping::belonging_to(&the_previous_dates)
                    .inner_join(schema::rooms::table)
                    .filter(not(schema::rooms::deleted))
                    .select(models::PreviousDateRoomMapping::as_select())
                    .load::<models::PreviousDateRoomMapping>(connection)?
                    .grouped_by(&the_previous_dates);

            let the_previous_dates = the_previous_dates
                .into_iter()
                .zip(the_previous_date_rooms)
                .map(
                    |(previous_date, previous_date_rooms)| models::FullPreviousDate {
                        previous_date,
                        room_ids: previous_date_rooms
                            .into_iter()
                            .map(|rm| rm.room_id)
                            .collect(),
                    },
                )
                .grouped_by(&the_entries);

            Ok(the_entries
                .into_iter()
                .zip(the_entry_rooms)
                .zip(the_previous_dates)
                .map(
                    |((entry, entry_rooms), entry_previous_dates)| models::FullEntry {
                        entry,
                        room_ids: entry_rooms.into_iter().map(|e| e.room_id).collect(),
                        previous_dates: entry_previous_dates,
                    },
                )
                .collect())
        })
    }

    fn get_entry(
        &mut self,
        auth_token: &AuthToken,
        entry_id: uuid::Uuid,
    ) -> Result<models::FullEntry, StoreError> {
        use diesel::dsl::not;
        use schema::entries::dsl::*;
        use schema::entry_rooms;
        use schema::previous_dates;
        use schema::rooms;

        self.connection.transaction(|connection| {
            let entry = entries
                .filter(id.eq(entry_id))
                .filter(not(deleted))
                .select(models::Entry::as_select())
                .first::<models::Entry>(connection)?;
            auth_token.check_privilege(entry.event_id, Privilege::ShowKueaPlan)?;

            let room_ids = entry_rooms::table
                .inner_join(rooms::table)
                .filter(entry_rooms::dsl::entry_id.eq(entry.id))
                .filter(not(rooms::deleted))
                .select(entry_rooms::dsl::room_id)
                .load::<uuid::Uuid>(connection)?;

            let previous_dates = previous_dates::table
                .filter(previous_dates::entry_id.eq(entry.id))
                .select(models::PreviousDate::as_select())
                .load::<models::PreviousDate>(connection)?;

            let the_previous_date_rooms =
                models::PreviousDateRoomMapping::belonging_to(&previous_dates)
                    .inner_join(schema::rooms::table)
                    .filter(not(schema::rooms::deleted))
                    .select(models::PreviousDateRoomMapping::as_select())
                    .load::<models::PreviousDateRoomMapping>(connection)?
                    .grouped_by(&previous_dates);

            Ok(models::FullEntry {
                entry,
                room_ids,
                previous_dates: previous_dates
                    .into_iter()
                    .zip(the_previous_date_rooms)
                    .map(
                        |(previous_date, previous_date_rooms)| models::FullPreviousDate {
                            previous_date,
                            room_ids: previous_date_rooms
                                .into_iter()
                                .map(|pdr| pdr.room_id)
                                .collect(),
                        },
                    )
                    .collect(),
            })
        })
    }

    fn create_or_update_entry(
        &mut self,
        auth_token: &AuthToken,
        entry: models::FullNewEntry,
        extend_previous_dates: bool,
        expected_last_update: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<bool, StoreError> {
        use diesel::dsl::not;
        use schema::entries::dsl::*;
        use schema::previous_dates;

        // The event_id of the existing entry is ensured to be the same (see below), so the
        // privilege level check holds for the existing and the new entry.
        auth_token.check_privilege(entry.entry.event_id, Privilege::ManageEntries)?;

        self.connection.transaction(|connection| {
            if let Some(expected_last_update) = expected_last_update {
                let actual_last_update = entries
                    .filter(id.eq(entry.entry.id))
                    .filter(not(deleted))
                    .select(last_updated)
                    .first::<chrono::DateTime<chrono::Utc>>(connection)?;
                if expected_last_update != actual_last_update {
                    return Err(StoreError::ConcurrentEditConflict);
                }
            }

            // entry
            let upsert_result = {
                // Unfortunately, `InsertStatement<_, OnConflictValues<...>>`, which is returned by
                // `.on_onflict().do_update()`, does not implement the QueryDsl trait for
                // `.filter()`, but only the `FilterDsl` trait directly. We import it locally here,
                // to not make the .filter() method in the following query ambiguous.
                use diesel::query_dsl::methods::FilterDsl;

                diesel::insert_into(entries)
                    .values(&entry.entry)
                    .on_conflict(id)
                    .do_update()
                    // By limiting the search of existing entries to the same event, we prevent
                    // changes of the event id (i.e. "moving" entries between events), which would
                    // be a security loophole
                    .set(&entry.entry)
                    .filter(event_id.eq(entry.entry.event_id))
                    .filter(not(deleted))
                    .returning(sql_upsert_is_updated())
                    .load::<bool>(connection)?
            };
            if upsert_result.is_empty() {
                return Err(StoreError::ConflictEntityExists);
            }
            let is_updated = upsert_result[0];

            // rooms
            update_entry_rooms(entry.entry.id, &entry.room_ids, connection)?;

            // previous dates
            if !extend_previous_dates {
                diesel::delete(
                    previous_dates::table
                        .filter(super::schema::previous_dates::entry_id.eq(entry.entry.id))
                        .filter(
                            previous_dates::id
                                .ne_all(entry.previous_dates.iter().map(|pd| pd.previous_date.id)),
                        ),
                )
                .execute(connection)?;
            }

            for previous_date in entry.previous_dates {
                update_or_insert_previous_date(&previous_date, entry.entry.id, connection)?
            }

            Ok(!is_updated)
        })
    }

    fn delete_entry(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        entry_id: uuid::Uuid,
    ) -> Result<(), StoreError> {
        use schema::entries::dsl::*;

        // The correctness of the given event_id is checked in the DELETE statement below
        auth_token.check_privilege(the_event_id, Privilege::ManageEntries)?;

        self.connection.transaction(|connection| {
            let count = diesel::update(entries)
                .filter(id.eq(entry_id))
                .filter(event_id.eq(the_event_id))
                .set(deleted.eq(true))
                .execute(connection)?;
            if count == 0 {
                return Err(StoreError::NotExisting);
            }

            Ok(())
        })
    }

    fn get_rooms(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: i32,
    ) -> Result<Vec<models::Room>, StoreError> {
        use diesel::dsl::not;
        use schema::rooms::dsl::*;
        auth_token.check_privilege(the_event_id, Privilege::ShowKueaPlan)?;

        Ok(rooms
            .select(models::Room::as_select())
            .filter(event_id.eq(the_event_id))
            .filter(not(deleted))
            .order_by(title)
            .load::<models::Room>(&mut self.connection)?)
    }

    fn create_or_update_room(
        &mut self,
        auth_token: &AuthToken,
        room: models::NewRoom,
    ) -> Result<bool, StoreError> {
        use diesel::dsl::not;
        use schema::rooms::dsl::*;

        // The event_id of the existing room is ensured to be the same (see below), so the
        // privilege level check holds for both, the existing and the new room.
        auth_token.check_privilege(room.event_id, Privilege::ManageRooms)?;

        let upsert_result = {
            // Unfortunately, `InsertStatement<_, OnConflictValues<...>>`, which is returned by
            // `.on_onflict().do_update()`, does not implement the QueryDsl trait for
            // `.filter()`, but only the `FilterDsl` trait directly. We import it locally here,
            // to not make the .filter() method in the following query ambiguous.
            use diesel::query_dsl::methods::FilterDsl;

            diesel::insert_into(rooms)
                .values(&room)
                .on_conflict(id)
                .do_update()
                // By limiting the search of existing rooms to the same event, we prevent changes
                // of the event id (i.e. "moving" entries between events), which would be a security
                // loophole
                .set(&room)
                .filter(event_id.eq(room.event_id))
                .filter(not(deleted))
                .returning(sql_upsert_is_updated())
                .load::<bool>(&mut self.connection)?
        };
        if upsert_result.is_empty() {
            return Err(StoreError::ConflictEntityExists);
        }
        let is_updated = upsert_result[0];
        Ok(!is_updated)
    }

    fn delete_room(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        room_id: uuid::Uuid,
        replace_with_rooms: &[RoomId],
        replace_with_room_comment: &str,
    ) -> Result<(), StoreError> {
        use schema::rooms::dsl::*;

        // The correctness of the given event_id is checked in the DELETE statement below
        auth_token.check_privilege(the_event_id, Privilege::ManageRooms)?;
        if !replace_with_rooms.is_empty() || !replace_with_room_comment.is_empty() {
            auth_token.check_privilege(the_event_id, Privilege::ManageEntries)?;
        }

        self.connection.transaction(|connection| {
            if !replace_with_room_comment.is_empty() {
                use schema::entries::dsl::*;

                diesel::update(entries)
                    .filter(exists(
                        schema::entry_rooms::table
                            .select(0.as_sql::<diesel::sql_types::Integer>())
                            .filter(schema::entry_rooms::entry_id.eq(id))
                            .filter(schema::entry_rooms::room_id.eq(room_id)),
                    ))
                    .set(
                        room_comment.eq(diesel::dsl::case_when(
                            room_comment.ne(""),
                            room_comment.concat("; "),
                        )
                        .otherwise("")
                        .concat(replace_with_room_comment)),
                    )
                    .execute(connection)?;
            }
            if !replace_with_rooms.is_empty() {
                replace_room_with_other_rooms(
                    the_event_id,
                    room_id,
                    replace_with_rooms,
                    connection,
                )?;
            }

            let count = diesel::update(rooms)
                .filter(id.eq(room_id))
                .filter(event_id.eq(the_event_id))
                .set(deleted.eq(true))
                .execute(connection)?;
            if count == 0 {
                return Err(StoreError::NotExisting);
            }

            Ok(())
        })
    }
    fn get_categories(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: i32,
    ) -> Result<Vec<models::Category>, StoreError> {
        use diesel::dsl::not;
        use schema::categories::dsl::*;
        auth_token.check_privilege(the_event_id, Privilege::ShowKueaPlan)?;

        Ok(categories
            .select(models::Category::as_select())
            .filter(event_id.eq(the_event_id))
            .filter(not(deleted))
            .order_by((sort_key, title))
            .load::<models::Category>(&mut self.connection)?)
    }

    fn create_or_update_category(
        &mut self,
        auth_token: &AuthToken,
        category: models::NewCategory,
    ) -> Result<bool, StoreError> {
        use diesel::dsl::not;
        use schema::categories::dsl::*;

        auth_token.check_privilege(category.event_id, Privilege::ManageCategories)?;

        let upsert_result = {
            // Unfortunately, `InsertStatement<_, OnConflictValues<...>>`, which is returned by
            // `.on_onflict().do_update()`, does not implement the QueryDsl trait for
            // `.filter()`, but only the `FilterDsl` trait directly. We import it locally here,
            // to not make the .filter() method in the following query ambiguous.
            use diesel::query_dsl::methods::FilterDsl;

            diesel::insert_into(categories)
                .values(&category)
                .on_conflict(id)
                .do_update()
                // By limiting the search of existing categories to the same event, we prevent
                // changes of the event id (i.e. "moving" categories between events), which would be
                // a security loophole
                .set(&category)
                .filter(event_id.eq(category.event_id))
                .filter(not(deleted))
                .returning(sql_upsert_is_updated())
                .load::<bool>(&mut self.connection)?
        };
        if upsert_result.is_empty() {
            return Err(StoreError::ConflictEntityExists);
        }
        let is_updated = upsert_result[0];
        Ok(!is_updated)
    }

    fn delete_category(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        category_id: uuid::Uuid,
        replacement_category: Option<CategoryId>,
    ) -> Result<(), StoreError> {
        use diesel::dsl::not;
        use schema::categories::dsl::*;

        // The correctness of the given event_id is checked in the DELETE statement below
        auth_token.check_privilege(the_event_id, Privilege::ManageCategories)?;
        if replacement_category.is_some() {
            auth_token.check_privilege(the_event_id, Privilege::ManageEntries)?;
        }

        self.connection.transaction(|connection| {
            let count_remaining_categories = categories
                .filter(event_id.eq(the_event_id))
                .filter(not(deleted))
                .filter(id.ne(category_id))
                .count()
                .execute(connection)?;
            if count_remaining_categories == 0 {
                return Err(StoreError::InvalidInputData(
                    "Cannot delete last category of the event.".to_owned(),
                ));
            };

            // Move entries to different category if requested
            if let Some(replacement_category) = replacement_category {
                use schema::entries::dsl::*;

                // Check that replacement actually exists in event
                let count = categories
                    .filter(schema::categories::id.eq(replacement_category))
                    .filter(schema::categories::event_id.eq(the_event_id))
                    .filter(not(schema::categories::deleted))
                    .count()
                    .execute(connection)?;
                if count == 0 {
                    return Err(StoreError::InvalidInputData(
                        "replacement category does not exist in event".into(),
                    ));
                };

                diesel::update(entries)
                    .filter(category.eq(category_id))
                    .filter(event_id.eq(the_event_id))
                    .set(category.eq(replacement_category))
                    .execute(connection)?;
            }

            let count = diesel::update(categories)
                .filter(id.eq(category_id))
                .filter(event_id.eq(the_event_id))
                .set(deleted.eq(true))
                .execute(connection)?;
            if count == 0 {
                return Err(StoreError::NotExisting);
            };

            Ok(())
        })
    }

    fn get_announcements(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        filter: Option<AnnouncementFilter>,
    ) -> Result<Vec<models::FullAnnouncement>, StoreError> {
        use diesel::dsl::not;
        use schema::announcements::dsl::*;
        auth_token.check_privilege(the_event_id, Privilege::ShowKueaPlan)?;

        self.connection.transaction(|connection| {
            let the_announcements = announcements
                .filter(event_id.eq(the_event_id))
                .filter(not(deleted))
                .filter(if let Some(filter) = filter {
                    announcement_filter_to_sql(filter)
                } else {
                    Box::new(diesel::dsl::sql::<diesel::sql_types::Bool>("TRUE"))
                })
                .order_by(sort_key)
                .select(models::Announcement::as_select())
                .load::<models::Announcement>(connection)?;

            let the_announcement_categories =
                models::AnnouncementCategoryMapping::belonging_to(&the_announcements)
                    .inner_join(schema::categories::table)
                    .filter(not(schema::categories::deleted))
                    .select(models::AnnouncementCategoryMapping::as_select())
                    .load::<models::AnnouncementCategoryMapping>(connection)?
                    .grouped_by(&the_announcements);

            let the_announcement_rooms =
                models::AnnouncementRoomMapping::belonging_to(&the_announcements)
                    .inner_join(schema::rooms::table)
                    .filter(not(schema::rooms::deleted))
                    .select(models::AnnouncementRoomMapping::as_select())
                    .load::<models::AnnouncementRoomMapping>(connection)?
                    .grouped_by(&the_announcements);

            Ok(the_announcements
                .into_iter()
                .zip(the_announcement_categories)
                .zip(the_announcement_rooms)
                .map(
                    |((announcement, announcement_categories), announcement_rooms)| {
                        models::FullAnnouncement {
                            announcement,
                            category_ids: announcement_categories
                                .into_iter()
                                .map(|e| e.category_id)
                                .collect(),
                            room_ids: announcement_rooms.into_iter().map(|e| e.room_id).collect(),
                        }
                    },
                )
                .collect())
        })
    }

    fn create_or_update_announcement(
        &mut self,
        auth_token: &AuthToken,
        announcement: models::FullNewAnnouncement,
        expected_last_update: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<bool, StoreError> {
        use diesel::dsl::not;
        use schema::announcements::dsl::*;

        // The event_id of the existing entry is ensured to be the same (see below), so the
        // privilege level check holds for the existing and the new entry.
        auth_token.check_privilege(
            announcement.announcement.event_id,
            Privilege::ManageAnnouncements,
        )?;

        self.connection.transaction(|connection| {
            if let Some(expected_last_update) = expected_last_update {
                let actual_last_update = announcements
                    .filter(id.eq(announcement.announcement.id))
                    .filter(not(deleted))
                    .select(last_updated)
                    .first::<chrono::DateTime<chrono::Utc>>(connection)?;
                if expected_last_update != actual_last_update {
                    return Err(StoreError::ConcurrentEditConflict);
                }
            }

            // announcement
            let upsert_result = {
                // Unfortunately, `InsertStatement<_, OnConflictValues<...>>`, which is returned by
                // `.on_onflict().do_update()`, does not implement the QueryDsl trait for
                // `.filter()`, but only the `FilterDsl` trait directly. We import it locally here,
                // to not make the .filter() method in the following query ambiguous.
                use diesel::query_dsl::methods::FilterDsl;

                diesel::insert_into(announcements)
                    .values(&announcement.announcement)
                    .on_conflict(id)
                    .do_update()
                    // By limiting the search of existing entries to the same event, we prevent
                    // changes of the event id (i.e. "moving" entries between events), which would
                    // be a security loophole
                    .set(&announcement.announcement)
                    .filter(event_id.eq(announcement.announcement.event_id))
                    .filter(not(deleted))
                    .returning(sql_upsert_is_updated())
                    .load::<bool>(connection)?
            };
            if upsert_result.is_empty() {
                return Err(StoreError::ConflictEntityExists);
            }
            let is_updated = upsert_result[0];

            update_announcement_categories(
                announcement.announcement.id,
                &announcement.category_ids,
                connection,
            )?;
            update_announcement_rooms(
                announcement.announcement.id,
                &announcement.room_ids,
                connection,
            )?;

            Ok(!is_updated)
        })
    }

    fn delete_announcement(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        announcement_id: AnnouncementId,
    ) -> Result<(), StoreError> {
        use schema::announcements::dsl::*;

        // The correctness of the given event_id is checked in the DELETE statement below
        auth_token.check_privilege(the_event_id, Privilege::ManageAnnouncements)?;

        self.connection.transaction(|connection| {
            let count = diesel::update(announcements)
                .filter(id.eq(announcement_id))
                .filter(event_id.eq(the_event_id))
                .set(deleted.eq(true))
                .execute(connection)?;
            if count == 0 {
                return Err(StoreError::NotExisting);
            }

            Ok(())
        })
    }

    fn authenticate_with_passphrase(
        &mut self,
        the_event_id: i32,
        the_passphrase: &str,
        session_token: &mut SessionToken,
    ) -> Result<(), StoreError> {
        use schema::event_passphrases::dsl::*;
        let passphrase_ids = event_passphrases
            .select(id)
            .filter(event_id.eq(the_event_id))
            .filter(passphrase.eq(the_passphrase))
            .load::<i32>(&mut self.connection)?;
        if !passphrase_ids.is_empty() {
            session_token.add_authorization(passphrase_ids[0]);
            Ok(())
        } else {
            Err(StoreError::NotExisting)
        }
    }

    fn get_auth_token_for_session(
        &mut self,
        session_token: &SessionToken,
        the_event_id: EventId,
    ) -> Result<AuthToken, StoreError> {
        use schema::event_passphrases::dsl::*;

        let role_ids = event_passphrases
            .select(privilege)
            .filter(event_id.eq(the_event_id))
            .filter(id.eq_any(session_token.get_passphrase_ids()))
            .load::<i32>(&mut self.connection)?;

        let mut roles = role_ids
            .iter()
            .map(|r| (*r).try_into())
            .collect::<Result<Vec<AccessRole>, EnumMemberNotExistingError>>()
            .map_err(|e| StoreError::InvalidDataInDatabase(e.to_string()))?;
        roles.sort_unstable();
        roles.dedup();

        Ok(AuthToken::create_for_session(the_event_id, roles))
    }

    fn create_reduced_session_token(
        &mut self,
        client_session_token: &SessionToken,
        the_event_id: EventId,
        expected_privilege: Privilege,
    ) -> Result<SessionToken, StoreError> {
        use schema::event_passphrases::dsl::*;

        let eligible_passphrase_ids =
            event_passphrases
                .select(id)
                .filter(event_id.eq(the_event_id))
                .filter(id.eq_any(client_session_token.get_passphrase_ids()).or(
                    derivable_from_passphrase.eq_any(client_session_token.get_passphrase_ids()),
                ))
                .filter(
                    privilege.eq_any(
                        expected_privilege
                            .qualifying_roles()
                            .iter()
                            .map(|r| *r as i32),
                    ),
                )
                .load::<i32>(&mut self.connection)?;
        if eligible_passphrase_ids.is_empty() {
            return Err(StoreError::NotExisting);
        }

        let mut result = SessionToken::new();
        result.add_authorization(eligible_passphrase_ids[0]);
        Ok(result)
    }
}

fn update_entry_rooms(
    the_entry_id: uuid::Uuid,
    room_ids: &[uuid::Uuid],
    connection: &mut PgConnection,
) -> Result<(), diesel::result::Error> {
    use schema::entry_rooms::dsl::*;

    diesel::delete(
        entry_rooms.filter(crate::data_store::schema::entry_rooms::dsl::entry_id.eq(the_entry_id)),
    )
    .execute(connection)?;

    diesel::insert_into(entry_rooms)
        .values(
            room_ids
                .iter()
                .map(|the_room_id| (entry_id.eq(the_entry_id), room_id.eq(the_room_id)))
                .collect::<Vec<_>>(),
        )
        .execute(connection)
        .map(|_| ())
}

fn update_or_insert_previous_date(
    previous_date: &models::FullPreviousDate,
    the_entry_id: EntryId,
    connection: &mut PgConnection,
) -> Result<(), StoreError> {
    use diesel::query_dsl::methods::FilterDsl;
    use schema::previous_dates::dsl::*;

    let result = diesel::insert_into(previous_dates)
        .values(&previous_date.previous_date)
        .on_conflict(id)
        .do_update()
        .set(&previous_date.previous_date)
        .filter(entry_id.eq(the_entry_id))
        .execute(connection)?;

    if result == 0 {
        return Err(StoreError::ConflictEntityExists);
    }

    update_previous_date_rooms(
        previous_date.previous_date.id,
        &previous_date.room_ids,
        connection,
    )?;

    Ok(())
}

fn update_previous_date_rooms(
    the_previous_date_id: uuid::Uuid,
    room_ids: &[uuid::Uuid],
    connection: &mut PgConnection,
) -> Result<(), diesel::result::Error> {
    use schema::previous_date_rooms::dsl::*;

    diesel::delete(previous_date_rooms.filter(previous_date_id.eq(the_previous_date_id)))
        .execute(connection)?;

    diesel::insert_into(previous_date_rooms)
        .values(
            room_ids
                .iter()
                .map(|the_room_id| {
                    (
                        previous_date_id.eq(the_previous_date_id),
                        room_id.eq(the_room_id),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .execute(connection)
        .map(|_| ())
}

fn update_announcement_categories(
    the_announcement_id: Uuid,
    category_ids: &[Uuid],
    connection: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<(), diesel::result::Error> {
    use schema::announcement_categories::dsl::*;

    diesel::delete(announcement_categories.filter(announcement_id.eq(the_announcement_id)))
        .execute(connection)?;

    diesel::insert_into(announcement_categories)
        .values(
            category_ids
                .iter()
                .map(|the_room_id| {
                    (
                        announcement_id.eq(the_announcement_id),
                        category_id.eq(the_room_id),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .execute(connection)
        .map(|_| ())
}

fn update_announcement_rooms(
    the_announcement_id: Uuid,
    room_ids: &[Uuid],
    connection: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<(), diesel::result::Error> {
    use schema::announcement_rooms::dsl::*;

    diesel::delete(announcement_rooms.filter(announcement_id.eq(the_announcement_id)))
        .execute(connection)?;

    diesel::insert_into(announcement_rooms)
        .values(
            room_ids
                .iter()
                .map(|the_room_id| {
                    (
                        announcement_id.eq(the_announcement_id),
                        room_id.eq(the_room_id),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .execute(connection)
        .map(|_| ())
}

fn replace_room_with_other_rooms(
    the_event_id: EventId,
    room_id: uuid::Uuid,
    replace_with_rooms: &[RoomId],
    connection: &mut PgConnection,
) -> Result<(), StoreError> {
    use diesel::dsl::not;
    use schema::entries;
    use schema::entry_rooms;
    use schema::rooms::dsl::*;

    // Check that replacements actually exists in event
    let count = rooms
        .filter(id.eq_any(replace_with_rooms))
        .filter(event_id.eq(the_event_id))
        .filter(not(deleted))
        .count()
        .execute(connection)?;
    if count != replace_with_rooms.len() {
        return Err(StoreError::InvalidInputData(
            "one of the replacement rooms does not exist in event".to_owned(),
        ));
    };

    let entry_ids: Vec<EntryId> = entry_rooms::table
        .filter(entry_rooms::room_id.eq(room_id))
        .select(entry_rooms::entry_id)
        .get_results(connection)?;
    diesel::delete(entry_rooms::table.filter(entry_rooms::room_id.eq(room_id)))
        .execute(connection)?;
    diesel::insert_into(entry_rooms::table)
        .values(
            entry_ids
                .iter()
                .flat_map(|entry_id| {
                    replace_with_rooms.iter().map(|room_id| {
                        (
                            entry_rooms::entry_id.eq(*entry_id),
                            entry_rooms::room_id.eq(*room_id),
                        )
                    })
                })
                .collect::<Vec<_>>(),
        )
        .execute(connection)?;
    diesel::update(entries::table)
        .set(entries::last_updated.eq(diesel::dsl::now))
        .execute(connection)?;
    Ok(())
}

type BoxedBoolExpression<'a, Table> =
    Box<dyn BoxableExpression<Table, diesel::pg::Pg, SqlType = diesel::sql_types::Bool> + 'a>;

fn entry_filter_to_sql<'a>(filter: EntryFilter) -> BoxedBoolExpression<'a, schema::entries::table> {
    use diesel::dsl::{exists, not};
    use schema::entries::dsl::*;

    let mut expression: BoxedBoolExpression<'a, schema::entries::table> =
        Box::new(diesel::dsl::sql::<diesel::sql_types::Bool>("TRUE"));
    if let Some(after) = filter.after {
        expression = Box::new(expression.as_expression().and(end.ge(after)));
    }
    if let Some(before) = filter.before {
        expression = Box::new(expression.as_expression().and(begin.lt(before)));
    }
    if let Some(rooms) = filter.rooms.clone() {
        expression = Box::new(
            expression.as_expression().and(exists(
                schema::entry_rooms::dsl::entry_rooms
                    .filter(schema::entry_rooms::entry_id.eq(id))
                    .filter(schema::entry_rooms::room_id.eq_any(rooms)),
            )),
        );
    }
    if filter.no_room {
        expression = Box::new(expression.as_expression().and(not(exists(
            schema::entry_rooms::dsl::entry_rooms.filter(schema::entry_rooms::entry_id.eq(id)),
        ))));
    }
    if filter.include_previous_date_matches
        && (filter.after.is_some() || filter.before.is_some() || filter.rooms.is_some())
    {
        use schema::previous_dates::dsl::*;
        let mut sub_query_filter: BoxedBoolExpression<'_, _> =
            Box::new(entry_id.eq(schema::entries::dsl::id));
        if let Some(after) = filter.after {
            sub_query_filter = Box::new(sub_query_filter.and(end.gt(after)));
        }
        if let Some(before) = filter.before {
            sub_query_filter = Box::new(sub_query_filter.and(begin.lt(before)));
        }
        if let Some(rooms) = filter.rooms {
            sub_query_filter = Box::new(
                sub_query_filter.as_expression().and(exists(
                    schema::previous_date_rooms::dsl::previous_date_rooms
                        .filter(schema::previous_date_rooms::previous_date_id.eq(id))
                        .filter(schema::previous_date_rooms::room_id.eq_any(rooms)),
                )),
            );
        }
        if filter.no_room {
            sub_query_filter = Box::new(
                sub_query_filter.as_expression().and(not(exists(
                    schema::previous_date_rooms::dsl::previous_date_rooms
                        .filter(schema::previous_date_rooms::previous_date_id.eq(id)),
                ))),
            );
        }
        expression = Box::new(
            expression.as_expression().or(exists(
                schema::previous_dates::table
                    .select(0.as_sql::<diesel::sql_types::Integer>())
                    .filter(sub_query_filter),
            )),
        );
    }
    if let Some(categories) = filter.categories {
        expression = Box::new(expression.as_expression().and(category.eq_any(categories)));
    }
    expression
}

fn announcement_filter_to_sql<'a>(
    filter: AnnouncementFilter,
) -> BoxedBoolExpression<'a, schema::announcements::table> {
    use diesel::dsl::exists;
    use schema::announcements::dsl::*;

    match filter {
        AnnouncementFilter::ForDate(date) => Box::new(
            show_with_days.and(
                begin_date
                    .is_null()
                    .or(begin_date.le(date).assume_not_null())
                    .and(end_date.is_null().or(end_date.ge(date).assume_not_null())),
            ),
        ),
        AnnouncementFilter::ForCategory(category_id) => Box::new(
            show_with_categories.and(
                show_with_all_categories.or(exists(
                    schema::announcement_categories::dsl::announcement_categories
                        .filter(schema::announcement_categories::announcement_id.eq(id))
                        .filter(schema::announcement_categories::category_id.eq(category_id)),
                )),
            ),
        ),
        AnnouncementFilter::ForRoom(room_id) => Box::new(
            show_with_rooms.and(
                show_with_all_rooms.or(exists(
                    schema::announcement_rooms::dsl::announcement_rooms
                        .filter(schema::announcement_rooms::announcement_id.eq(id))
                        .filter(schema::announcement_rooms::room_id.eq(room_id)),
                )),
            ),
        ),
    }
}
