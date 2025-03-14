use super::{
    models, schema, AccessRole, AuthToken, EntryFilter, EnumMemberNotExistingError, EventId,
    GlobalAuthToken, KuaPlanStore, KueaPlanStoreFacade, StoreError,
};
use crate::auth_session::SessionToken;
use diesel::expression::AsExpression;
use diesel::pg::PgConnection;
use diesel::prelude::*;

#[derive(Clone)]
pub struct PgDataStore {
    pool: diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>>,
}

impl PgDataStore {
    pub fn new(database_url: &str) -> Result<Self, String> {
        let connection_manager = diesel::r2d2::ConnectionManager::<PgConnection>::new(database_url);
        Ok(Self {
            pool: diesel::r2d2::Pool::builder()
                .test_on_check_out(true)
                .build(connection_manager)
                .map_err(|e| format!("Could not create database connection pool: {}", e))?,
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
    fn get_event(
        &mut self,
        _auth_token: &AuthToken,
        event_id: i32,
    ) -> Result<models::Event, StoreError> {
        use schema::events::dsl::*;

        events
            .filter(id.eq(event_id))
            .first::<models::Event>(&mut self.connection)
            .map_err(|e| e.into())
    }

    fn create_event(
        &mut self,
        auth_token: &GlobalAuthToken,
        event: models::NewEvent,
    ) -> Result<i32, StoreError> {
        use schema::events::dsl::*;
        auth_token.check_privilege(AccessRole::Admin)?;

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
        auth_token.check_privilege(the_event_id, AccessRole::User)?;

        self.connection.transaction(|connection| {
            let the_entries = entries
                .filter(event_id.eq(the_event_id))
                .filter(not(deleted))
                .filter(filter_to_sql(filter))
                .order_by((begin.asc(), end.asc()))
                .load::<models::Entry>(connection)?;

            let the_entry_rooms = models::EntryRoomMapping::belonging_to(&the_entries)
                .load::<models::EntryRoomMapping>(connection)?
                .grouped_by(&the_entries);

            Ok(the_entries
                .into_iter()
                .zip(the_entry_rooms)
                .map(|(entry, entry_rooms)| models::FullEntry {
                    entry,
                    room_ids: entry_rooms.into_iter().map(|e| e.room_id).collect(),
                })
                .collect())
        })
    }

    fn get_entry(
        &mut self,
        auth_token: &AuthToken,
        entry_id: uuid::Uuid,
    ) -> Result<models::FullEntry, StoreError> {
        use schema::entries::dsl::*;
        use schema::entry_rooms;

        self.connection.transaction(|connection| {
            let entry = entries
                .filter(id.eq(entry_id))
                .first::<models::Entry>(connection)?;
            auth_token.check_privilege(entry.event_id, AccessRole::User)?;

            if entry.deleted {
                return Err(StoreError::NotExisting);
            }

            let room_ids = entry_rooms::table
                .filter(entry_rooms::dsl::entry_id.eq(entry.id))
                .select(entry_rooms::dsl::room_id)
                .load::<uuid::Uuid>(connection)?;

            Ok(models::FullEntry { entry, room_ids })
        })
    }

    fn create_or_update_entry(
        &mut self,
        auth_token: &AuthToken,
        entry: models::FullNewEntry,
    ) -> Result<bool, StoreError> {
        use diesel::dsl::not;
        use schema::entries::dsl::*;
        use schema::entry_rooms;

        // The event_id of the existing entry is ensured to be the same (see below), so the
        // privilege level check holds for the existing and the new entry.
        auth_token.check_privilege(entry.entry.event_id, AccessRole::Orga)?;

        self.connection.transaction(|connection| {
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

            diesel::delete(
                entry_rooms::table.filter(entry_rooms::dsl::entry_id.eq(entry.entry.id)),
            )
            .execute(connection)?;
            insert_entry_rooms(entry.entry.id, &entry.room_ids, connection)?;

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
        auth_token.check_privilege(the_event_id, AccessRole::Orga)?;

        let count = diesel::update(entries)
            .filter(id.eq(entry_id))
            .filter(event_id.eq(the_event_id))
            .set(deleted.eq(true))
            .execute(&mut self.connection)?;
        if count == 0 {
            return Err(StoreError::NotExisting);
        }

        Ok(())
    }

    fn get_rooms(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: i32,
    ) -> Result<Vec<models::Room>, StoreError> {
        use diesel::dsl::not;
        use schema::rooms::dsl::*;
        auth_token.check_privilege(the_event_id, AccessRole::User)?;

        Ok(rooms
            .select(models::Room::as_select())
            .filter(event_id.eq(the_event_id))
            .filter(not(deleted))
            .load::<models::Room>(&mut self.connection)?)
    }

    fn create_or_update_room(
        &mut self,
        auth_token: &AuthToken,
        room: models::NewRoom,
    ) -> Result<bool, StoreError> {
        use schema::rooms::dsl::*;

        // The event_id of the existing room is ensured to be the same (see below), so the
        // privilege level check holds for both, the existing and the new room.
        auth_token.check_privilege(room.event_id, AccessRole::Orga)?;

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
                .filter(deleted.eq(false))
                .returning(sql_upsert_is_updated())
                .load::<bool>(&mut self.connection)?
        };
        if upsert_result.is_empty() {
            return Err(StoreError::NotExisting);
        }
        let is_updated = upsert_result[0];
        Ok(!is_updated)
    }

    fn delete_room(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        room_id: uuid::Uuid,
    ) -> Result<(), StoreError> {
        use schema::rooms::dsl::*;

        // The correctness of the given event_id is checked in the DELETE statement below
        auth_token.check_privilege(the_event_id, AccessRole::Orga)?;

        let count = diesel::update(rooms)
            .filter(id.eq(room_id))
            .filter(event_id.eq(the_event_id))
            .set(deleted.eq(true))
            .execute(&mut self.connection)?;
        if count == 0 {
            return Err(StoreError::NotExisting);
        }

        Ok(())
    }

    fn get_categories(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: i32,
    ) -> Result<Vec<models::Category>, StoreError> {
        use schema::categories::dsl::*;
        auth_token.check_privilege(the_event_id, AccessRole::User)?;

        Ok(categories
            .select(models::Category::as_select())
            .filter(event_id.eq(the_event_id))
            .filter(deleted.eq(false))
            .load::<models::Category>(&mut self.connection)?)
    }

    fn create_or_update_category(
        &mut self,
        auth_token: &AuthToken,
        category: models::NewCategory,
    ) -> Result<bool, StoreError> {
        use schema::categories::dsl::*;

        auth_token.check_privilege(category.event_id, AccessRole::Orga)?;

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
                .filter(deleted.eq(false))
                .returning(sql_upsert_is_updated())
                .load::<bool>(&mut self.connection)?
        };
        if upsert_result.is_empty() {
            return Err(StoreError::NotExisting);
        }
        let is_updated = upsert_result[0];
        Ok(!is_updated)
    }

    fn delete_category(
        &mut self,
        auth_token: &AuthToken,
        the_event_id: EventId,
        category_id: uuid::Uuid,
    ) -> Result<(), StoreError> {
        use schema::categories::dsl::*;

        // The correctness of the given event_id is checked in the DELETE statement below
        auth_token.check_privilege(the_event_id, AccessRole::Orga)?;

        let count = diesel::update(categories)
            .filter(id.eq(category_id))
            .filter(event_id.eq(the_event_id))
            .set(deleted.eq(true))
            .execute(&mut self.connection)?;
        if count == 0 {
            return Err(StoreError::NotExisting);
        }

        Ok(())
    }

    fn authorize(
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

    fn check_authorization(
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
            .collect::<Result<Vec<AccessRole>, EnumMemberNotExistingError>>()?;
        let implied_roles = roles
            .iter()
            .flat_map(|e| e.implied_roles())
            .copied()
            .collect::<Vec<_>>();
        roles.extend(implied_roles);
        roles.sort_unstable();
        roles.dedup();

        Ok(AuthToken {
            event_id: the_event_id,
            roles,
        })
    }
}

fn insert_entry_rooms(
    the_entry_id: uuid::Uuid,
    room_ids: &[uuid::Uuid],
    connection: &mut PgConnection,
) -> Result<(), diesel::result::Error> {
    use schema::entry_rooms::dsl::*;

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

type BoxedBoolExpression<'a> = Box<
    dyn BoxableExpression<schema::entries::table, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>
        + 'a,
>;

fn filter_to_sql<'a>(filter: EntryFilter) -> BoxedBoolExpression<'a> {
    use diesel::dsl::{exists, not};
    use schema::entries::dsl::*;

    let mut expression: BoxedBoolExpression<'a> =
        Box::new(diesel::dsl::sql::<diesel::sql_types::Bool>("TRUE"));
    if let Some(after) = filter.after {
        expression = Box::new(expression.as_expression().and(end.ge(after)));
    }
    if let Some(before) = filter.before {
        expression = Box::new(expression.as_expression().and(begin.le(before)));
    }
    if let Some(categories) = filter.categories {
        expression = Box::new(expression.as_expression().and(category.eq_any(categories)));
    }
    if let Some(rooms) = filter.rooms {
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
    expression
}
