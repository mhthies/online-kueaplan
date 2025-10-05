
# online-kueaplan

A web application server for serving an online KüA-Plan, written in Rust, using a Postgresql database and providing a server-side dynamic web user interface and a REST API.

## Historic Background

Originally, the online-kueaplan was designed with a client-side frontend / Progressive Web Application to provide improved offline functionality.
The server-side web application would only serve static files and provide the REST api for modifying entries in the database and synchronizing the frontend's state with the latest changes in the database.
The client-side frontend would either be based on a modern JavaScript/TypeScript framework or a Rust web frontend framework (using WASM).

Thus, a comprehensive REST API has been defined and implemented.
In addition, the Rust structure definitions corresponding to the JSON object schemas of the API specification have been declared in a separate Rust crate, to be reused in the client-side frontend code.

In the meantime, due to immediate need, a very simplistic online KüA-Plan application has been implemented and used at SommerAkademie 2024: The [simple-kueaplan](https://tracker.cde-ev.de/gitea/michaelthies/simple_kueaplan).
It is implemented in Python, uses server-side template rendering with Jinja and has no database backend, but is fed from YAML files in a Git repository.
This system was well received by the event's participants, but the YAML files were cumbersome to use for the orgas.

In order to get the database-backed Rust KüA-Plan application to a usable state as fast as possible, a server-side user interface has been added, using the fundamental UI design of the *simple-kueaplan*.

The client-side frontend may be added any time later.
Regardless, the existing REST API can be used for programmatic interaction with the KüA-Plan application.

## Structure and Tech Stack

### Server Application
The main server-side web application is a self-contained Rust crate *kueaplan_server*, living in the `server/` directory.

It uses the [Diesel](https://diesel.rs/) framework for database access, [Actix Web](https://actix.rs/) as the web frontend framework and [Askama](https://docs.rs/askama/latest/askama/) for HTML template rendering.

The user interface is based on [Bootstrap 5.3](https://getbootstrap.com/docs/5.3/getting-started/introduction/).
For some pages of the user interface, client-side helper functions have been added, using vanilla JavaScript.
Most notably, [TomSelect](https://tom-select.js.org/) is used for creating multi-select inputs.

The static frontend artifacts from the `server/static/` are embedded into the compiled server executable, to form a self-contained executable.
Same holds for the database migration SQL files from `server/migrations/`.
The Askama template files in `server/templates` are converted to Rust code at compile time by Askama's derive macro.

The application's Rust code is split into the following major modules:
* `data_store`: abstract datastore interface, Rust models of the database entities and Diesel-based Postgresql implementation of the interface
* `web.api`: REST API endpoints
* `web.ui`: manifold framework and helper code for the building the user interface
* `web.ui.endpoints`: server-side web user interface endpoints
* `cli`: command-line interface commands (other than `serve`).


### REST API

The formal OpenAPI specification of the REST API, including JSON schemas, can be found in `etc/spec`.

Rust representations of the JSON schemas, annotated with the appropriate Serde attributes for use with serde_json are implemented in the separate Rust crate *kueaplan_api_types* in `api_types/`.
This crate is used by the server application for serializing and deserializing data in the REST API endpoints.


## Deployment and Usage

### Postgresql Database

The kueaplan_server requires access to a single Postgresql database, on which it has CONNECT, USAGE, SELECT, INSERT, UPDATE and DELETE privileges.
Running the database schema migrations (see below) requires CREATE, TRIGGER, EXECUTE and REFERENCES privileges in addition.
The database should be created with an appropriate unicode encoding and collation.

Database schema migrations (including initial creationg) are *not* automatically executed at application startup.
This allows to use a separate database user/role with reduced privileges for operation.
However, the application checks the migration state at startup and refuses to start with an outdated database schema.

Setup example on a hosted Postgresql server:
```sql
CREATE DATABASE "kueaplan"
  WITH OWNER "kueaplan"
  ENCODING 'UTF8'
  LC_COLLATE = 'de_DE.UTF-8'
  LC_CTYPE = 'de_DE.UTF-8';
```

### Configuration and Running
The kueaplan_server is configured through environment variables.
Alternatively, the environment variables can be defined in a `.env` file, localed in the server's working directory.

The following environment variables are available.
**All of them are mandatory.**

| envrionment variable | example                                               | description                                                                                  |
|----------------------|-------------------------------------------------------|----------------------------------------------------------------------------------------------|
| DATABASE_URL         | postgresql://username:password@localhost/databasename |                                                                                              |
| SECRET               |                                                       | true-random secret string, only known to the sever, which is used for symmetric cryptography |
| LISTEN_PORT          | 9000                                                  | HTTP listening port                                                                          |
| LISTEN_ADDRESS       | ::1                                                   | HTTP listen address. Use `::` for listening on all IPv4 and IPv6 interfaces.                 |
| ADMIN_NAME           | Anton Administrator                                   | displayed name of the admin of this instance (for error messages, etc.)                      |
| ADMIN_EMAIL          | mail@example.com                                      | displayed email address of the admin of this instance (for error messages, etc.)             |

To start the server, run
```bash
kueaplan_server serve
```

The server runs as a simple (foreground) command line application.
There is no "daemon mode", etc.
It can be stopped gracefully with a simple SIGTERM.
Use your favorite service manager to run it as a daemon service (recommanded: systemd. See below).

### Database Schema Migration

The `kuaeplan_server` has built-in functionality for initializing and the database schema and updating it to the current version.
However, these database schema migrations are *not* automatically executed at application startup.
Instead, the application checks the migration state at startup and refuses to start with an outdated database schema.

To execute all pending database schema migrations, run
```bash
kueaplan_server migrate-database
```
This command requires the configuration environment variables to be provided as environment or `.env` file (see above).
In particular, it uses the `DATABASE_URL` to select the Postgresql database to be migrated.


### Systemd Unit

For production use, it is recommended to run the `kueaplan_server` as a systemd service.

The following service file may be a good starting point.
It assumes that the `kueaplan_server` executable is placed in `/usr/local/bin` and  there is a specific user and group `kueaplan` for running the server.
In addition, there needs to be a configuration file, defining the environment variables at `/etc/kueaplan/env`.
(Make sure to chose appropriate access permissions on that file, since it contains the application SECRET and probably the database user password.)

```ini
[Unit]
Description=Online KüA-Plan web application server
After=network.target postgresql.service
Requires=network.target postgresql.service

[Service]
Type=simple
User=kueaplan
Group=kueaplan
ExecStart=/usr/local/bin/kueaplan_server serve
EnvironmentFile=/etc/kueaplan/env
Restart=always

[Install]
WantedBy=multi-user.target
```


### Reverse Proxy
The kueaplan_server can be run behind a HTTP reverse proxy server, e.g. for virtual host discrimination and TLS termination.

For the apache web server, the following virtual host configuration can be used as a starting point:
```
<VirtualHost *:443>
        ServerName kueaplan.de
        ServerAdmin mail@example.com

        ProxyRequests Off
        ProxyPreserveHost on

        AllowEncodedSlashes NoDecode
        ProxyPass / http://localhost:9000/ nocanon
        ProxyPassReverse / http://localhost:9000/
        RequestHeader set X-Forwarded-Proto https

        # TODO SSL setup
</VirtualHost>
```


## Development

### API Spec

Besides other tools, the [OpenAPI Generator](https://openapi-generator.tech/) can be used to generate Code or documentation from the API specification files.
This can be used to check the specification's syntax and semantics.
For generating an API documentation in HTML format, the command in `etc/spec/gendocs.sh` can be used.


### Code formatting

- Rust code must be properly formatted, using rustfmt's default settings.
  Run `cargo fmt` to fix formatting before committing!
- All files should be free of trailing whitespace and end with a single line-feed.


#### pre-commit Framework for Git Hooks

The pre-commit framework [link](https://pre-commit.com/) can be used to automatically check and fix the code formatting before committing.
To use it:
* Install pre-commit (e.g. using `uv tool install pre-commit`)
* Run `pre-commit install` in this repository to install the Git Hooks according to the `.pre-commit-config.yaml`.


### Unit Tests and System Tests

Running the Rust unittests:
```bash
cargo test
```

Running the system tests, written in Python: See [tests/README.md](tests/README.md).
