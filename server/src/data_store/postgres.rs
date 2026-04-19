use super::{
    models, schema, AnnouncementFilter, AnnouncementId, CategoryId, DataPolicy, EntryFilter,
    EntryId, EventFilter, EventId, KuaPlanStore, KueaPlanStoreFacade, PassphraseId, PreviousDateId,
    RoomId, StoreError,
};
use crate::auth_session::SessionToken;
use crate::data_store::auth_token::{AccessRole, AuthToken, GlobalAuthToken, Privilege};
use diesel::expression::AsExpression;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::PooledConnection;
use std::collections::HashMap;
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
    fn get_events(&mut self, filter: EventFilter) -> Result<Vec<models::Event>, StoreError> {
        use schema::events::dsl::*;

        events
            .filter(event_filter_to_sql(filter))
            .order_by((begin_date, end_date, id))
            .select(models::Event::as_select())
            .load::<models::Event>(&mut self.connection)
            .map_err(|e| e.into())
    }

    fn get_event(&mut self, event_id: i32) -> Result<models::Event, StoreError> {
        use schema::events::dsl::*;

        events
            .filter(id.eq(event_id))
            .select(models::Event::as_select())
            .first::<models::Event>(&mut self.connection)
            .map_err(|e| e.into())
    }

    fn get_event_by_slug(&mut self, event_slug: &str) -> Result<models::Event, StoreError> {
        use schema::events::dsl::*;

        events
            .filter(slug.eq(event_slug))
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
        event: models::ExtendedEvent,
    ) -> Result<i32, StoreError> {
        use schema::events::dsl::*;
        auth_token.check_privilege(Privilege::CreateEvents)?;

        Ok(diesel::insert_into(events)
            .values(&event)
            .returning(id)
            .get_result::<EventId>(&mut self.connection)?)
    }

    fn update_event(
        &mut self,
        auth_token: &AuthToken,
        event: models::ExtendedEvent,
    ) -> Result<(), StoreError> {
        use schema::events::dsl::*;
        auth_token.check_privilege(event.basic_data.id, Privilege::EditEventDetails)?;

        event
            .default_time_schedule
            .validate(event.clock_info.effective_begin_of_day)
            .map_err(StoreError::InvalidInputData)?;

        let result = diesel::update(events)
            .filter(id.eq(event.basic_data.id))
            .set(event)
            .execute(&mut self.connection)?;
        if result == 1 {
            Ok(())
        } else {
            Err(StoreError::NotExisting)
        }
    }

    fn delete_event(
        &mut self,
        auth_token: &AuthToken,
        event_id: EventId,
    ) -> Result<(), StoreError> {
        use schema::events::dsl::*;
        auth_token.check_privilege(event_id, Privilege::DeleteEvents)?;
        let rows = diesel::delete(events)
            .filter(id.eq(event_id))
            .execute(&mut self.connection)?;
        if rows == 0 {
            return Err(StoreError::NotExisting);
        }
        Ok(())
    }

    fn import_event_with_contents(
        &mut self,
        auth_token: &GlobalAuthToken,
        data: models::EventWithContents,
    ) -> Result<EventId, StoreError> {
        self.connection.transaction(|connection| {
            let event_id = {
                use schema::events::dsl::*;
                auth_token.check_privilege(Privilege::CreateEvents)?;

                diesel::insert_into(events)
                    .values(data.event)
                    .returning(id)
                    .get_result::<EventId>(connection)?
            };

            let mut rooms = data.rooms;
            for room in rooms.iter_mut() {
                room.event_id = event_id;
            }
            diesel::insert_into(schema::rooms::table)
                .values(rooms)
                .execute(connection)?;

            let mut categories = data.categories;
            for category in categories.iter_mut() {
                category.event_id = event_id;
            }
            diesel::insert_into(schema::categories::table)
                .values(categories)
                .execute(connection)?;

            for full_entry in data.entries {
                let mut entry = full_entry.entry;
                let entry_id = entry.id;
                entry.event_id = event_id;
                check_categories_validity(&[entry.category], event_id, connection)?;
                diesel::insert_into(schema::entries::table)
                    .values(entry)
                    .execute(connection)?;
                check_rooms_validity(&full_entry.room_ids, event_id, connection)?;
                update_entry_rooms(entry_id, &full_entry.room_ids, connection)?;
                for previous_date in full_entry.previous_dates {
                    check_rooms_validity(&previous_date.room_ids, event_id, connection)?;
                    update_or_insert_previous_date(&previous_date, entry_id, connection)?;
                }
            }

            for full_announcement in data.announcements {
                let mut announcement = full_announcement.announcement;
                let announcement_id = announcement.id;
                announcement.event_id = event_id;
                check_categories_validity(&full_announcement.category_ids, event_id, connection)?;
                check_rooms_validity(&full_announcement.room_ids, event_id, connection)?;
                diesel::insert_into(schema::announcements::table)
                    .values(announcement)
                    .execute(connection)?;
                update_announcement_categories(
                    announcement_id,
                    &full_announcement.category_ids,
                    connection,
                )?;
                update_announcement_rooms(
                    announcement_id,
                    &full_announcement.room_ids,
                    connection,
                )?;
            }

            Ok(event_id)
        })
    }

    fn get_published_entries_filtered(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: i32,
        filter: EntryFilter,
    ) -> Result<Vec<models::FullEntry>, StoreError> {
        auth_token.check_privilege(the_event_id, Privilege::ShowKueaPlan)?;
        get_entries_generic(
            &mut self.connection,
            the_event_id,
            filter,
            models::EntryState::all().filter(|s| s.is_published()),
            false,
        )
    }

    fn get_all_entries_filtered(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        filter: EntryFilter,
        state_filter: &[models::EntryState],
    ) -> Result<Vec<models::FullEntry>, StoreError> {
        auth_token.check_privilege(the_event_id, Privilege::ManageEntries)?;
        get_entries_generic(
            &mut self.connection,
            the_event_id,
            filter,
            state_filter.iter(),
            true,
        )
    }

    fn get_entry_count_by_state(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
    ) -> Result<Vec<(models::EntryState, i64)>, StoreError> {
        use diesel::dsl::{count_star, not};
        use schema::entries::dsl::*;

        auth_token.check_privilege(the_event_id, Privilege::ManageEntries)?;

        let result = entries
            .group_by(state)
            .select((state, count_star()))
            .filter(event_id.eq(the_event_id))
            .filter(not(deleted))
            .load::<(models::EntryState, i64)>(&mut self.connection)?;
        Ok(result)
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
            if !entry.state.is_published() {
                auth_token.check_privilege(entry.event_id, Privilege::ManageEntries)?;
            }

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

            let orga_internal = auth_token
                .has_privilege(entry.event_id, Privilege::ManageEntries)
                .then(|| {
                    entries
                        .filter(id.eq(entry_id))
                        .select(models::EntryInternalFields::as_select())
                        .first::<models::EntryInternalFields>(connection)
                })
                .transpose()?;

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
                orga_internal,
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

            check_categories_validity(&[entry.entry.category], entry.entry.event_id, connection)?;

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
            check_rooms_validity(&entry.room_ids, entry.entry.event_id, connection)?;
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
                check_rooms_validity(&previous_date.room_ids, entry.entry.event_id, connection)?;
                update_or_insert_previous_date(&previous_date, entry.entry.id, connection)?;
            }

            Ok(!is_updated)
        })
    }

    fn patch_entry(
        &mut self,
        auth_token: &AuthToken,
        entry_id: EntryId,
        entry_data: models::EntryPatch,
    ) -> Result<(), StoreError> {
        use schema::entries::dsl::*;

        self.connection.transaction(|connection| {
            let current_event_id = entries
                .select(event_id)
                .filter(id.eq(entry_id))
                .first::<EventId>(connection)?;

            auth_token.check_privilege(current_event_id, Privilege::ManageEntries)?;

            if let Some(room_ids) = entry_data.room_ids.as_ref() {
                check_rooms_validity(room_ids, current_event_id, connection)?;
                update_entry_rooms(entry_id, room_ids, connection)?;
            }
            if let Some(category_id) = entry_data.category.as_ref() {
                check_categories_validity(&[*category_id], current_event_id, connection)?;
            }
            diesel::update(entries)
                .filter(id.eq(entry_id))
                .set((entry_data, last_updated.eq(diesel::dsl::now)))
                .execute(connection)?;

            Ok(())
        })
    }

    fn submit_entry_by_participant(
        &mut self,
        auth_token: &AuthToken,
        entry: models::FullNewEntry,
    ) -> Result<(), StoreError> {
        use schema::entries::dsl::*;

        auth_token.check_privilege(entry.entry.event_id, Privilege::SubmitParticipantEntries)?;

        self.connection.transaction(|connection| {
            let event_data = schema::events::table
                .filter(schema::events::id.eq(entry.entry.event_id))
                .select(models::ExtendedEvent::as_select())
                .first::<models::ExtendedEvent>(connection)?;
            check_categories_validity(&[entry.entry.category], entry.entry.event_id, connection)?;
            check_submission_policies(&entry, connection, event_data.entry_submission_mode)?;

            diesel::insert_into(entries)
                .values(&entry.entry)
                .execute(connection)?;

            // rooms
            check_rooms_validity(&entry.room_ids, entry.entry.event_id, connection)?;
            update_entry_rooms(entry.entry.id, &entry.room_ids, connection)?;

            for previous_date in entry.previous_dates {
                check_rooms_validity(&previous_date.room_ids, entry.entry.event_id, connection)?;
                update_or_insert_previous_date(&previous_date, entry.entry.id, connection)?;
            }

            Ok(())
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

    fn create_or_update_previous_date(
        &mut self,
        auth_token: &AuthToken,
        previous_date: models::FullPreviousDate,
    ) -> Result<bool, StoreError> {
        self.connection.transaction(|connection| {
            // Check if referenced entry exists and get entry's event_id for auth check
            let event_id = schema::entries::table
                .filter(schema::entries::id.eq(previous_date.previous_date.entry_id))
                .select(schema::entries::event_id)
                .first::<EventId>(connection)?;

            auth_token.check_privilege(event_id, Privilege::ManageEntries)?;
            check_rooms_validity(&previous_date.room_ids, event_id, connection)?;

            let created = update_or_insert_previous_date(
                &previous_date,
                previous_date.previous_date.entry_id,
                connection,
            )?;
            Ok(created)
        })
    }

    fn delete_previous_date(
        &mut self,
        auth_token: &AuthToken,
        entry_id: EntryId,
        previous_date_id: PreviousDateId,
    ) -> Result<(), StoreError> {
        use schema::entries;

        self.connection.transaction(|connection| {
            // Check if referenced entry exists and get entry's event_id for auth check
            let event_id = schema::entries::table
                .filter(schema::entries::id.eq(entry_id))
                .select(schema::entries::event_id)
                .first::<EventId>(connection)?;

            auth_token.check_privilege(event_id, Privilege::ManageEntries)?;

            diesel::delete(
                schema::previous_dates::table
                    .filter(schema::previous_dates::entry_id.eq(entry_id))
                    .filter(schema::previous_dates::id.eq(previous_date_id)),
            )
            .execute(connection)?;
            // We need to somehow mark the entry as changed, so clients using the sync API will be
            // informed about the change. Since the previous_date itself does no longer exist, we
            // cannot use it's last_updated field for this purpose, anymore.
            diesel::update(entries::table)
                .filter(entries::id.eq(entry_id))
                .set(entries::last_updated.eq(diesel::dsl::now))
                .execute(connection)?;
            Ok(())
        })
    }

    fn get_entry_count_by_category(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
    ) -> Result<Vec<(CategoryId, i64)>, StoreError> {
        use diesel::dsl::{count_star, not};
        use schema::entries::dsl::*;

        auth_token.check_privilege(the_event_id, Privilege::ShowKueaPlan)?;
        Ok(entries
            .filter(event_id.eq(the_event_id))
            .filter(not(deleted))
            .filter(not(is_cancelled))
            .group_by(category)
            .select((category, count_star()))
            .load::<(CategoryId, i64)>(&mut self.connection)?)
    }

    fn get_entry_count_by_room(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
    ) -> Result<Vec<(RoomId, i64)>, StoreError> {
        use diesel::dsl::{count_star, not};
        use schema::entries::dsl::*;

        auth_token.check_privilege(the_event_id, Privilege::ShowKueaPlan)?;
        Ok(entries
            .inner_join(schema::entry_rooms::table)
            .filter(event_id.eq(the_event_id))
            .filter(not(deleted))
            .filter(not(is_cancelled))
            .group_by(schema::entry_rooms::room_id)
            .select((schema::entry_rooms::room_id, count_star()))
            .load::<(RoomId, i64)>(&mut self.connection)?)
    }

    fn get_entry_count_without_room(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
    ) -> Result<i64, StoreError> {
        use diesel::dsl::{count_star, exists, not};
        use schema::entries::dsl::*;

        auth_token.check_privilege(the_event_id, Privilege::ShowKueaPlan)?;
        Ok(entries
            .filter(event_id.eq(the_event_id))
            .filter(not(deleted))
            .filter(not(is_cancelled))
            .filter(not(exists(
                schema::entry_rooms::table.filter(schema::entry_rooms::entry_id.eq(id)),
            )))
            .select(count_star())
            .first::<i64>(&mut self.connection)?)
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
        use diesel::dsl::exists;
        use schema::rooms::dsl::*;
        use schema::{announcement_rooms, announcements};

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

            let count = diesel::update(rooms)
                .filter(id.eq(room_id))
                .filter(event_id.eq(the_event_id))
                .set(deleted.eq(true))
                .execute(connection)?;
            if count == 0 {
                return Err(StoreError::NotExisting);
            }

            // do this after we marked the room as deleted, to make sure that we detect when you
            // replace with the room itself. This is fine, because we're working in a database
            // transaction.
            replace_room_with_other_rooms(the_event_id, room_id, replace_with_rooms, connection)?;

            // update announcements for the deleted room
            diesel::update(announcements::table)
                .filter(exists(
                    announcement_rooms::table
                        .select(0.as_sql::<diesel::sql_types::Integer>())
                        .filter(announcement_rooms::announcement_id.eq(announcements::id))
                        .filter(announcement_rooms::room_id.eq(room_id)),
                ))
                .set(announcements::last_updated.eq(diesel::dsl::now))
                .execute(connection)?;
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
        use diesel::dsl::{exists, not};
        use schema::categories::dsl::*;
        use schema::{announcement_categories, announcements};

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
                .first::<i64>(connection)?;
            if count_remaining_categories == 0 {
                return Err(StoreError::InvalidInputData(
                    "Cannot delete last category of the event.".to_owned(),
                ));
            };

            let count = diesel::update(categories)
                .filter(id.eq(category_id))
                .filter(event_id.eq(the_event_id))
                .set(deleted.eq(true))
                .execute(connection)?;
            if count == 0 {
                return Err(StoreError::NotExisting);
            };

            // Move entries to different category if requested
            // Do this after we marked the room as deleted, to make sure that we detect when you
            // replace with the room itself. This is fine, because we're working in a database
            // transaction.
            if let Some(replacement_category) = replacement_category {
                use schema::entries::dsl::*;

                check_categories_validity(&[replacement_category], the_event_id, connection)?;

                diesel::update(entries)
                    .filter(category.eq(category_id))
                    .filter(event_id.eq(the_event_id))
                    .set(category.eq(replacement_category))
                    .execute(connection)?;
            } else {
                // Otherwise, make sure that there are no entries in this category
                let count_entries = schema::entries::table
                    .filter(schema::entries::category.eq(category_id))
                    .filter(not(schema::entries::deleted))
                    .count()
                    .first::<i64>(connection)?;
                if count_entries != 0 {
                    return Err(StoreError::InvalidInputData(
                        "The category is still referenced by entries.".to_owned(),
                    ));
                };
            }

            // update announcements for the deleted category
            // We do not actually delete the announcement_categories references here. This is not
            // necessary, because the related query function (get_announcements()) filters out
            // references to deleted categories.
            // On the other hand, keeping the references allows for recovery from an accidental
            // deletion of a category.
            diesel::update(announcements::table)
                .filter(exists(
                    announcement_categories::table
                        .select(0.as_sql::<diesel::sql_types::Integer>())
                        .filter(announcement_categories::announcement_id.eq(announcements::id))
                        .filter(announcement_categories::category_id.eq(category_id)),
                ))
                .set(announcements::last_updated.eq(diesel::dsl::now))
                .execute(connection)?;

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
            check_categories_validity(
                &announcement.category_ids,
                announcement.announcement.event_id,
                connection,
            )?;
            check_rooms_validity(
                &announcement.room_ids,
                announcement.announcement.event_id,
                connection,
            )?;

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

    fn patch_announcement(
        &mut self,
        auth_token: &AuthToken,
        announcement_id: AnnouncementId,
        announcement_data: models::AnnouncementPatch,
    ) -> Result<(), StoreError> {
        use schema::announcements::dsl::*;

        self.connection.transaction(|connection| {
            let current_event_id = announcements
                .select(event_id)
                .filter(id.eq(announcement_id))
                .first::<EventId>(connection)?;

            auth_token.check_privilege(current_event_id, Privilege::ManageAnnouncements)?;

            if let Some(room_ids) = announcement_data.room_ids.as_ref() {
                check_categories_validity(room_ids, current_event_id, connection)?;
                update_announcement_rooms(announcement_id, room_ids, connection)?;
            }
            if let Some(category_ids) = announcement_data.category_ids.as_ref() {
                check_rooms_validity(category_ids, current_event_id, connection)?;
                update_announcement_categories(announcement_id, category_ids, connection)?;
            }
            diesel::update(announcements)
                .filter(id.eq(announcement_id))
                .set((announcement_data, last_updated.eq(diesel::dsl::now)))
                .execute(connection)?;
            Ok(())
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
        let passphrase_ids_and_validity = event_passphrases
            .select((id, valid_from, valid_until))
            .filter(event_id.eq(the_event_id))
            .filter(passphrase.eq(the_passphrase))
            .load::<(
                i32,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<chrono::DateTime<chrono::Utc>>,
            )>(&mut self.connection)?;
        if passphrase_ids_and_validity.is_empty() {
            return Err(StoreError::NotExisting);
        }

        let now = chrono::Utc::now();
        let valid_passphrases: Vec<i32> = passphrase_ids_and_validity
            .into_iter()
            .filter(|(_pid, begin, end)| {
                begin.is_none_or(|b| b <= now) && end.is_none_or(|e| e >= now)
            })
            .map(|(pid, _, _)| pid)
            .collect();
        if valid_passphrases.is_empty() {
            return Err(StoreError::NotValid);
        }
        session_token.add_authorization(valid_passphrases[0]);
        Ok(())
    }

    fn drop_access_role(
        &mut self,
        the_event_id: i32,
        access_role: AccessRole,
        session_token: &mut SessionToken,
    ) -> Result<(), StoreError> {
        use schema::event_passphrases::dsl::*;
        let passphrase_ids = event_passphrases
            .select(id)
            .filter(event_id.eq(the_event_id))
            .filter(privilege.eq(access_role))
            .load::<i32>(&mut self.connection)?;

        for passphrase_id in passphrase_ids {
            session_token.remove_authorization(passphrase_id);
        }

        Ok(())
    }

    fn list_all_access_roles(
        &mut self,
        session_token: &SessionToken,
    ) -> Result<Vec<(EventId, AccessRole)>, StoreError> {
        use schema::event_passphrases::dsl::*;

        let mut roles = event_passphrases
            .filter(id.eq_any(session_token.get_passphrase_ids()))
            .filter(valid_from.is_null().or(valid_from.le(diesel::dsl::now)))
            .filter(valid_until.is_null().or(valid_until.ge(diesel::dsl::now)))
            .select((event_id, privilege))
            .load::<(EventId, AccessRole)>(&mut self.connection)?;

        roles.sort_unstable();
        roles.dedup();
        roles.retain(|(_event, role)| role.can_be_granted_by_passphrase());

        Ok(roles)
    }

    fn get_auth_token_for_session(
        &mut self,
        session_token: &SessionToken,
        the_event_id: EventId,
    ) -> Result<AuthToken, StoreError> {
        use schema::event_passphrases::dsl::*;

        let data = event_passphrases
            .select((privilege, valid_from, valid_until))
            .filter(event_id.eq(the_event_id))
            .filter(id.eq_any(session_token.get_passphrase_ids()))
            .load::<(
                AccessRole,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<chrono::DateTime<chrono::Utc>>,
            )>(&mut self.connection)?;

        let now = chrono::Utc::now();
        let mut roles = Vec::new();
        let mut expired_roles = Vec::new();
        for (role, begin, end) in data {
            if begin.is_none_or(|b| b <= now) && end.is_none_or(|e| e >= now) {
                roles.push(role);
            } else {
                expired_roles.push(role);
            }
        }
        roles.sort_unstable();
        roles.dedup();
        expired_roles.sort_unstable();
        expired_roles.dedup();
        // special roles like [AccessRole::ServerAdmin] must never be given to web/API user
        roles.retain(|role| role.can_be_granted_by_passphrase());

        Ok(AuthToken::create_for_session(
            the_event_id,
            roles,
            expired_roles,
        ))
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

    fn create_passphrase(
        &mut self,
        auth_token: &AuthToken,
        passphrase: models::NewPassphrase,
    ) -> Result<PassphraseId, StoreError> {
        auth_token.check_privilege(passphrase.event_id, Privilege::ManagePassphrases)?;
        if !(passphrase.privilege.can_be_managed_online()
            || auth_token.has_privilege(passphrase.event_id, Privilege::ManageSecurePassphrases))
        {
            return Err(StoreError::InvalidInputData(format!(
                "Cannot create a passphrase with access role {:?} via the web interface.",
                passphrase.privilege
            )));
        }
        if !passphrase.privilege.can_be_granted_by_passphrase() {
            return Err(StoreError::InvalidInputData(format!(
                "Cannot create a passphrase with special access role {:?}.",
                passphrase.privilege
            )));
        }

        let result = diesel::insert_into(schema::event_passphrases::table)
            .values(passphrase)
            .returning(schema::event_passphrases::id)
            .get_result::<PassphraseId>(&mut self.connection)?;
        Ok(result)
    }

    fn patch_passphrase(
        &mut self,
        auth_token: &AuthToken,
        passphrase_id: PassphraseId,
        passphrase_data: models::PassphrasePatch,
    ) -> Result<(), StoreError> {
        use schema::event_passphrases::dsl::*;

        self.connection.transaction(|connection| {
            let current_event_id = event_passphrases
                .select(event_id)
                .filter(id.eq(passphrase_id))
                .first::<EventId>(connection)?;

            auth_token.check_privilege(current_event_id, Privilege::ManagePassphrases)?;

            diesel::update(event_passphrases)
                .filter(id.eq(passphrase_id))
                .set(passphrase_data)
                .execute(connection)?;
            Ok(())
        })
    }

    fn delete_passphrase(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        passphrase_id: PassphraseId,
    ) -> Result<(), StoreError> {
        use schema::event_passphrases::dsl::*;

        // correctness of event_id is checked in DELETE statement below
        auth_token.check_privilege(the_event_id, Privilege::ManagePassphrases)?;

        self.connection.transaction(|connection| {
            diesel::update(event_passphrases)
                .filter(event_id.eq(the_event_id))
                .filter(derivable_from_passphrase.eq(passphrase_id))
                .set(derivable_from_passphrase.eq(None::<i32>))
                .execute(connection)?;

            let affected_rows = diesel::delete(event_passphrases)
                .filter(id.eq(passphrase_id))
                .filter(event_id.eq(the_event_id))
                // Admin passphrases cannot be deleted via the web UI and API
                .filter(privilege.eq_any(AccessRole::all().filter(|x| {
                    auth_token.has_privilege(the_event_id, Privilege::ManageSecurePassphrases)
                        || x.can_be_managed_online()
                })))
                .execute(connection)?;
            if affected_rows > 0 {
                Ok(())
            } else {
                Err(StoreError::NotExisting)
            }
        })
    }

    fn get_passphrases(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
    ) -> Result<Vec<models::Passphrase>, StoreError> {
        use schema::event_passphrases::dsl::*;
        auth_token.check_privilege(the_event_id, Privilege::ManagePassphrases)?;

        let mut passphrases = event_passphrases
            .select(models::Passphrase::as_select())
            .filter(event_id.eq(the_event_id))
            .order_by(privilege)
            .load::<models::Passphrase>(&mut self.connection)?;
        for p in passphrases.iter_mut() {
            p.passphrase = p.passphrase.as_ref().map(|x| obfuscate_passphrase(x));
        }
        Ok(passphrases)
    }
    fn get_full_user_passphrases(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
    ) -> Result<Vec<models::Passphrase>, StoreError> {
        use schema::event_passphrases::dsl::*;
        auth_token.check_privilege(the_event_id, Privilege::ManagePassphrases)?;

        let passphrases = event_passphrases
            .select(models::Passphrase::as_select())
            .filter(event_id.eq(the_event_id))
            .filter(passphrase.is_not_null())
            .filter(privilege.eq(AccessRole::User))
            .load::<models::Passphrase>(&mut self.connection)?;
        Ok(passphrases)
    }
}

fn get_entries_generic<'a, StateIter: Iterator<Item = &'a models::EntryState>>(
    connection: &mut PgConnection,
    the_event_id: EventId,
    filter: EntryFilter,
    state_filter: StateIter,
    with_internal_fields: bool,
) -> Result<Vec<models::FullEntry>, StoreError> {
    use diesel::dsl::not;
    use schema::entries::dsl::*;

    connection.transaction(|connection| {
        let the_entries = entries
            .filter(event_id.eq(the_event_id))
            .filter(not(deleted))
            .filter(state.eq_any(state_filter))
            .filter(entry_filter_to_sql(filter))
            .order_by((begin.asc(), end.asc(), id.asc()))
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

        let mut the_entries = the_entries
            .into_iter()
            .zip(the_entry_rooms)
            .zip(the_previous_dates)
            .map(
                |((entry, entry_rooms), entry_previous_dates)| models::FullEntry {
                    entry,
                    room_ids: entry_rooms.into_iter().map(|e| e.room_id).collect(),
                    previous_dates: entry_previous_dates,
                    orga_internal: None,
                },
            )
            .collect::<Vec<_>>();

        if with_internal_fields {
            let entry_index_by_id: HashMap<_, _> = the_entries
                .iter()
                .enumerate()
                .map(|(i, u)| (u.entry.id, i))
                .collect();

            let entries_internal_fields = entries
                .filter(id.eq_any(the_entries.iter().map(|e| e.entry.id)))
                .select((id, models::EntryInternalFields::as_select()))
                .load::<(EntryId, models::EntryInternalFields)>(connection)?;

            for (entry_id, internal_fields) in entries_internal_fields {
                the_entries[*entry_index_by_id.get(&entry_id).unwrap()].orga_internal =
                    Some(internal_fields);
            }
        }

        Ok(the_entries)
    })
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
) -> Result<bool, StoreError> {
    use diesel::query_dsl::methods::FilterDsl;
    use schema::previous_dates::dsl::*;

    let upsert_result = diesel::insert_into(previous_dates)
        .values(&previous_date.previous_date)
        .on_conflict(id)
        .do_update()
        .set(&previous_date.previous_date)
        .filter(entry_id.eq(the_entry_id))
        .returning(sql_upsert_is_updated())
        .load::<bool>(connection)?;
    if upsert_result.is_empty() {
        return Err(StoreError::ConflictEntityExists);
    }
    let is_updated = upsert_result[0];

    update_previous_date_rooms(
        previous_date.previous_date.id,
        &previous_date.room_ids,
        connection,
    )?;

    Ok(!is_updated)
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
    use schema::entries;
    use schema::entry_rooms;
    use schema::previous_date_rooms;
    use schema::previous_dates;

    // Check that replacements actually exists in event
    check_rooms_validity(replace_with_rooms, the_event_id, connection)?;

    // We do not actually delete the entry_rooms, previous_date_rooms and announcement_rooms
    // reference entries here. This is not necessary, because the related query functions
    // (get_entries_filtered(), get_entry() and get_announcements) filter out references to deleted
    // rooms.
    // On the other hand, keeping the references allows for recovery from an accidental deletion of
    // a room.

    let entry_ids: Vec<EntryId> = entry_rooms::table
        .filter(entry_rooms::room_id.eq(room_id))
        .select(entry_rooms::entry_id)
        .get_results(connection)?;
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
        .filter(entries::id.eq_any(entry_ids))
        .set(entries::last_updated.eq(diesel::dsl::now))
        .execute(connection)?;

    let previous_date_ids: Vec<EntryId> = previous_date_rooms::table
        .filter(previous_date_rooms::room_id.eq(room_id))
        .select(previous_date_rooms::previous_date_id)
        .get_results(connection)?;
    diesel::insert_into(previous_date_rooms::table)
        .values(
            previous_date_ids
                .iter()
                .flat_map(|previous_date_id| {
                    replace_with_rooms.iter().map(|room_id| {
                        (
                            previous_date_rooms::previous_date_id.eq(*previous_date_id),
                            previous_date_rooms::room_id.eq(*room_id),
                        )
                    })
                })
                .collect::<Vec<_>>(),
        )
        .execute(connection)?;
    diesel::update(previous_dates::table)
        .filter(previous_dates::id.eq_any(previous_date_ids))
        .set(previous_dates::last_updated.eq(diesel::dsl::now))
        .execute(connection)?;
    Ok(())
}

fn check_categories_validity(
    category_ids: &[CategoryId],
    given_event_id: EventId,
    connection: &mut PgConnection,
) -> Result<(), StoreError> {
    use schema::categories::dsl::*;
    let result = categories
        .filter(id.eq_any(category_ids))
        .select((id, event_id, deleted))
        .load::<(CategoryId, EventId, bool)>(connection)?;
    // We don't need to check for existence here, since this is done by the foreign key constraint
    for (category_id, category_event_id, category_deleted) in result {
        if category_deleted {
            return Err(StoreError::InvalidInputData(format!(
                "Category {category_id} has been deleted."
            )));
        }
        if category_event_id != given_event_id {
            return Err(StoreError::InvalidInputData(format!(
                "Category {category_id} does not belong to event {given_event_id}."
            )));
        }
    }
    Ok(())
}

fn check_rooms_validity(
    room_ids: &[RoomId],
    the_event_id: EventId,
    connection: &mut PgConnection,
) -> Result<(), StoreError> {
    use schema::rooms::dsl::*;
    let result = rooms
        .filter(id.eq_any(room_ids))
        .select((id, event_id, deleted))
        .load::<(RoomId, EventId, bool)>(connection)?;
    // We don't need to check for existence here, since this is done by the foreign key constraint
    for (room_id, room_event_id, room_deleted) in result {
        if room_deleted {
            return Err(StoreError::InvalidInputData(format!(
                "Room {room_id} has been deleted."
            )));
        }
        if room_event_id != the_event_id {
            return Err(StoreError::InvalidInputData(format!(
                "Room {room_id} does not belong to event {the_event_id}."
            )));
        }
    }
    Ok(())
}

/// Check if the given entry can be submitted by a participant, i.e. it does not use orga-only
/// features or creates conflicts with other entries.
///
/// * category is not an "official" category
/// * no room conflicts
/// * no exclusive entry conflicts
/// * no usage of orga-only entry properties: exclusive entry
fn check_submission_policies(
    entry: &models::FullNewEntry,
    connection: &mut PgConnection,
    submission_mode: models::EntrySubmissionMode,
) -> Result<(), StoreError> {
    if !submission_mode.allows_entry_submission() {
        return Err(StoreError::PolicyViolation(
            DataPolicy::EntrySubmissionEnabled,
        ));
    }
    if !submission_mode
        .allowed_submission_states()
        .contains(&entry.entry.state)
    {
        return Err(StoreError::PolicyViolation(
            DataPolicy::EntrySubmissionReviewState,
        ));
    }

    if entry.entry.is_exclusive {
        return Err(StoreError::PolicyViolation(
            DataPolicy::EntrySubmissionNoExclusiveProperty,
        ));
    }

    let is_official_category = schema::categories::table
        .filter(schema::categories::id.eq(entry.entry.category))
        .select(schema::categories::is_official)
        .first::<bool>(connection)
        .map_err(|e| -> StoreError {
            match e {
                // when the category does not exist, we should return the same error as the database
                // constraint violation would create later
                diesel::result::Error::NotFound => StoreError::InvalidInputData(
                    "Entry's category must reference an existing category.".to_owned(),
                ),
                e => e.into(),
            }
        })?;
    if is_official_category {
        return Err(StoreError::PolicyViolation(
            DataPolicy::EntrySubmissionNoOfficialCategory,
        ));
    }

    let conflicts_base_query = schema::entries::table
        .filter(schema::entries::event_id.eq(entry.entry.event_id))
        .filter(diesel::dsl::not(schema::entries::deleted))
        .filter(diesel::dsl::not(schema::entries::is_cancelled))
        .filter(
            schema::entries::state.eq_any(models::EntryState::all().filter(|s| s.is_published())),
        )
        .filter(schema::entries::begin.lt(entry.entry.end))
        .filter(schema::entries::end.gt(entry.entry.begin));

    let exclusive_conflicts = conflicts_base_query
        .clone()
        .filter(schema::entries::is_exclusive)
        .select(diesel::dsl::count_star())
        .first::<i64>(connection)?;
    if exclusive_conflicts > 0 {
        return Err(StoreError::PolicyViolation(
            DataPolicy::EntrySubmissionNoExclusiveConflict,
        ));
    }

    let room_conflicts = conflicts_base_query
        .filter(diesel::dsl::exists(
            schema::entry_rooms::table
                .filter(schema::entry_rooms::entry_id.eq(schema::entries::id))
                .filter(schema::entry_rooms::room_id.eq_any(&entry.room_ids)),
        ))
        .select(diesel::dsl::count_star())
        .first::<i64>(connection)?;
    if room_conflicts > 0 {
        return Err(StoreError::PolicyViolation(
            DataPolicy::EntrySubmissionNoRoomConflict,
        ));
    }
    Ok(())
}

type BoxedBoolExpression<'a, Table> =
    Box<dyn BoxableExpression<Table, diesel::pg::Pg, SqlType = diesel::sql_types::Bool> + 'a>;

fn event_filter_to_sql<'a>(filter: EventFilter) -> BoxedBoolExpression<'a, schema::events::table> {
    use schema::events::dsl::*;

    let mut expression: BoxedBoolExpression<'a, schema::events::table> =
        Box::new(diesel::dsl::sql::<diesel::sql_types::Bool>("TRUE"));
    if let Some(after) = filter.after {
        expression = Box::new(expression.as_expression().and(end_date.ge(after)));
    }
    if let Some(before) = filter.before {
        expression = Box::new(expression.as_expression().and(begin_date.lt(before)));
    }
    expression
}

fn entry_filter_to_sql<'a>(filter: EntryFilter) -> BoxedBoolExpression<'a, schema::entries::table> {
    use diesel::dsl::{exists, not};
    use schema::entries::dsl::*;

    let mut expression: BoxedBoolExpression<'a, schema::entries::table> =
        Box::new(diesel::dsl::sql::<diesel::sql_types::Bool>("TRUE"));
    if let Some(after) = filter.after {
        expression = if filter.after_inclusive {
            Box::new(expression.as_expression().and(end.ge(after)))
        } else {
            Box::new(expression.as_expression().and(end.gt(after)))
        };
    }
    if let Some(before) = filter.before {
        expression = if filter.before_inclusive {
            Box::new(expression.as_expression().and(begin.le(before)))
        } else {
            Box::new(expression.as_expression().and(begin.lt(before)))
        };
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
            sub_query_filter = if filter.after_inclusive {
                Box::new(sub_query_filter.and(end.ge(after)))
            } else {
                Box::new(sub_query_filter.and(end.gt(after)))
            };
        }
        if let Some(before) = filter.before {
            sub_query_filter = if filter.before_inclusive {
                Box::new(sub_query_filter.and(begin.le(before)))
            } else {
                Box::new(sub_query_filter.and(begin.lt(before)))
            };
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

/// Replace some characters of the passphrase with <DEL> characters to allow the user to recognize
/// the passphrase without leaking it completely.
fn obfuscate_passphrase(value: &str) -> String {
    let length = value.chars().count();
    let num_clear_chars = length.div_ceil(5);
    let num_obfuscated_chars = length - num_clear_chars;
    std::iter::repeat_n('\x7f', num_obfuscated_chars)
        .chain(value.chars().skip(num_obfuscated_chars))
        .collect()
}

/// Get a human-readable description of the consistency expectation that is checked by a specific
/// constraint in our Postgres database schema by the constraint's name.
///
/// These are visible to the user when creating or updating entities inconsistently via the REST
/// API.
///
/// Returns None, when no human-readable description is present of the given constraint name. This
/// may be the case when we don't expect this constraint to be violated by a user interaction.
pub fn description_for_postgres_constraint(constraint_name: &str) -> Option<&'static str> {
    match constraint_name {
        "announcement_categories_category_id_fkey" => Some("Announcement's categories must reference existing categories."),
        "announcement_rooms_room_id_fkey" => Some("Announcement's rooms must reference existing rooms."),
        "announcements_date_range" => Some("Announcement's begin_date must be earlier or equal to end_date."),
        "entries_category_fkey" => Some("Entry's category must reference an existing category."),
        "entries_time_range" => Some("Entry's begin must be earlier or equal to end."),
        "entry_rooms_room_id_fkey" => Some("Entry's rooms must reference existing rooms."),
        "event_passphrases_derivable_from_passphrase_fkey" => Some("Passphrase's derivable_from_passphrase must be null or reference an existing passphrase."),
        "events_preceding_event_id_fkey" => Some("Event's preceding_event_id must be null or reference an existing event."),
        "events_subsequent_event_id_fkey" => Some("Event's subsequent_event_id must be null or reference an existing event."),
        "events_date_range" => Some("Event's begin_date must be earlier or equal to end_date."),
        "previous_date_rooms_room_id_fkey" => Some("PreviousDate's rooms must reference existing rooms."),
        "previous_dates_time_range" => Some("PreviousDate's begin must be earlier or equal to end."),
        _ => None,
    }
}
