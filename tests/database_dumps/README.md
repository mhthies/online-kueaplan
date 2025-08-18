Postgres Database dumps used for running systemtests.
The testing database is automatically restored to one of the dumped states before each individual test by the `reset_database` Fixture (see conftest.py).
The dump to be used can be controlled by the pytest marker *database_dump*.

New dumps can be created from a running database with the `pg_dump` command:
```shell
pg_dump -c -f dump.sql --no-owner -U <dbuser> <dbname>
```

Dumps can be manually restored with
```shell
psql -f dump.sql <database_url>
```
