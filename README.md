
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

TODO


## Development

### Server Application

TODO

### API Spec

Besides other tools, the [OpenAPI Generator](https://openapi-generator.tech/) can be used to generate Code or documentation from the API specification files.
This can be used to check the specification's syntax and semantics.
For generating an API documentation in HTML format, the command in `etc/spec/gendocs.sh` can be used. 
