# System Tests

This directory contains a suite of system tests for the kueaplan web application, written in Python with pytest.
For UI tests, the Playwright web browser automation framework is used.
For REST API tests, a Python API client is generated from the API spec at ../etc/spec/openapi.json using openapi-generator.

## Running the tests

### Preliminaries

* Set up *Postgresql* like for production usage.
  Create a separate database for the automated system tests.
  See [main README.md](../README.md) for details.
  The database user, used for the system tests, must have all migration and usage privileges on the database, i.e. CONNECT, USAGE, SELECT, INSERT, UPDATE, DELETE, CREATE, TRIGGER, EXECUTE and REFERENCES.
  You don't need to run the database schema migrations, since the tests will automatically initialize the database from SQL dumps.

* Create a Python virtualenv and install the requirements from `requirements.txt`.

  Recommended way, using the `uv` package manager:
  ```bash
  uv venv
  uv pip install -r requirements.txt
  ```
  Alternative way, using pip:
  ```bash
  python -m virtualenv .venv
  ./.venv/bin/pip install -r requirements
  ```

* Run Playwright installation.
  This installs the browser executables required by playwright to a global cache directory in your user home directory (`~/.cache/ms-playwright/` on Linux systems).

  When using `uv`:
  ```bash
  uv run playwright install
  ```
  Alternatively, directly run the `playwright` executable directly from the virtualenv: `./.venv/bin/playwright install`.

* Make sure the Postgresql command-line client `psql` is available in the $PATH.

* Optional: Install `openapi-generator-cli`, such that it is available in the $PATH.
  See https://openapi-generator.tech/docs/installation/ for instructions.
  If openapi-generator-cli is not available, the API tests are skipped.


### Test Execution

TL;DR
```bash
export DATABASE_URL=postgres://user:password@localhost/test_database_name
uv run pytest --browser chromium --video retain-on-failure --start-app
```

Execute `pytest` from the virtualenv in this directory to run all system tests.
See https://docs.pytest.org/en/6.2.x/usage.html for additional command-line options, e.g. for running only specific tests.

The database connection information for the test database needs to specified via the `DATABASE_URL` environment variable in the same format as `kueaplan_server` expects it.
This allows the test framework to restore the database to a known state for each individual test.

To enable the Playwright UI tests, the browser(s) need to be selected.
In addition, the Playwright option `--video retain-on-failure` is helpful for analyzing failing UI tests.
See https://playwright.dev/python/docs/test-runners for a full reference of pytest-playwright command-line options.


#### Starting the kueplan_server

The system tests are run against a normally built `kueplan_server` executable.
The server can either be started manually before starting the tests (e.g. for using a debugger) or it can be managed automatically by the system tests framework.

To let the pytest tests manage the `kueaplan_server` executable, add the parameter `--start-app` to the `pytest` command.

Starting and stopping the server is then handled by the `start_kueaplan_server()` pytest fixture from `conftest.py`.
The server is automatically built using `cargo build` and started with appropriate environment variables.

When starting the `kueaplan_server` manually, make sure to set the following environment variables:
* `DATABASE_URL` same as for the `pytest` command
* `LISTEN_PORT=9099`



#### Using pytest.ini and .env files

For simplifying the invocation of the pytest command, you can specify the command-line arguments in a `pytest.ini` file, instead of supplying them for each invocation.
`pytest.ini` contents (in this directory):
```text
[pytest]
addopts = --browser chromium --video retain-on-failure --start-app
```

In addition, a `.env` file can be used to specify `DATABASE_URL` variable instead of setting it as a real environment variable.

`.env` contents (located in this directory):
```text
DATABASE_URL=postgres://user:password@localhost/test_database_name
```

## Utilities

### Updating the database schema of the database dumps

When new Diesel database migrations are created, the database dumps in `database_dumps/` need to be updated.
Otherwise, the kueaplan_server won't run on the test database.
To update all database dumps automatically, the `migrate_database_dumps.py` script can be used:
```bash
export DATABASE_URL=postgres://user:password@localhost/test_database_name
uv run python migrate_database_dumps.py
```
The script requires `pg_dump` and `diesel` executables to be available in the PATH, in addition to `psql` (which is also required for running the test; see above).
Like the PyTest tests, the script uses a `.env` file, if available.
This allows to omit the `DATABASE_URL` variable in the environment.
