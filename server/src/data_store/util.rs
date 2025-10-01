use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use diesel::query_builder::bind_collector::RawBytesBindCollector;
use diesel::serialize::ToSql;
use diesel::{AsExpression, FromSqlRow};

/// Helper struct to allow storin a chrono_tz timezone in the database (as string identifier of the
/// timezone). This struct implements `ToSql<VarChar, _>` and `FromSql<VarChar, _>`.
#[derive(Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type=diesel::sql_types::Text)]
pub struct TimezoneWrapper(chrono_tz::Tz);

impl<DB> ToSql<diesel::sql_types::Text, DB> for TimezoneWrapper
where
    DB: diesel::backend::Backend,
    for<'c> &'c str: ToSql<diesel::sql_types::Text, DB>,
    for<'c> DB: Backend<BindCollector<'c> = RawBytesBindCollector<DB>>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        let value = self.0.name();
        <str as ToSql<diesel::sql_types::Text, DB>>::to_sql(value, &mut out.reborrow())
    }
}

impl<DB> FromSql<diesel::sql_types::Text, DB> for TimezoneWrapper
where
    DB: diesel::backend::Backend,
    String: FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(
        bytes: <DB as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        let x = String::from_sql(bytes)?;
        let tz = x
            .parse()
            .map_err(|x| format!("Invalid timezone name: {}", x))?;
        Ok(Self(tz))
    }
}

impl From<TimezoneWrapper> for chrono_tz::Tz {
    fn from(value: TimezoneWrapper) -> Self {
        value.0
    }
}

impl From<chrono_tz::Tz> for TimezoneWrapper {
    fn from(value: chrono_tz::Tz) -> Self {
        Self(value)
    }
}
