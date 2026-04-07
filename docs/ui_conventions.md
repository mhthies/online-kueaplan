# UX and UI Design Conventions

## Buttons

- links that initiate an action with persistent effects, SHOULD be styled as a button (`.btn`), e.g. navigation to edit form, navigation to deletion page
- links/buttons that don't immediately perform an action, but only navigate to the next page, MUST be styled as Outline-Button (`.btn-outline-...`), e.g. navigation to edit form
- links/buttons that _do_ immediately perform an action with persistent effects, MUST be styled as filled button (not Outline button), e.g. submit button of edit form, login button
- all buttons SHOULD have an icon

- buttons related to destructive operations (deleting/removing information), SHOULD be colored in red (`.btn-danger`)
- buttons related to creation of new entities, SHOULD be colored in green (`.btn-success`)
- buttons for navigating back to the previous page and/or aborting the current action should be color in grey (`.btn-outline-secondary`)

## Icons

### Actions
| Usage                       | Icon                       |
|-----------------------------|----------------------------|
| submit form to save changes | `bi-save`                  |
| log in                      | `bi-box-arrow-in-right`    |
| abort current action        | `bi-x-square`              |
| go to some relevant page    | `bi-arrow-up-right-circle` |
| add entity                  | `bi-plus-lg`               |
| add secondary entity        | `bi-plus-circle`           |
| delete entity               | `bi-trash`                 |
| edit entity                 | `bi-pencil`                |
| hide                        | `bi-eye-slash`             |
| mark as cancelled/disabled  | `bi-x-cirle`               |
| accept/publish entry        | `bi-check2-circle`         |
| (no state change)           | `bi-circle`                |
| reject                      | `bi-slash-circle`          |
| reload                      | `bi-arrow-clockwise`       |
| navigate back               | `bi-arrow-left`            |

### Entities
| Usage                                        | Icon                      |
|----------------------------------------------|---------------------------|
| date (general/effective date)                | `bi-calendar`             |
| calendar date (as opposed to effective date) | `bi-calendar-event-fill`  |
| place                                        | `bi-geo-alt-fill`         |
| category                                     | `bi-grid`                 |
| announcement                                 | `bi-chat-right-text-fill` |
| previous date                                | `bi-clock-history`        |

### Properties
| Usage                                      | Icon                   |
|--------------------------------------------|------------------------|
| visibility                                 | `bi-eye`               |
| true                                       | `bi-check-circle-fill` |
| false                                      | `bi-x-circle`          |
| actual current date of entry               | `bi-clock-fill`        |
| orga-internal comment                      | `bi-chat-right-dots`   |
| entry state: draft                         | `bi-pencil-square`     |
| entry state: requires review               | `bi-clipboard2-check`  |
| entry state: rejected                      | `bi-slash-circle`      |
| entry state: retracted                     | `bi-eye-slash`         |
| passphrase access role: user               | `bi-person-fill`       |
| passphrase access role: orga               | `bi-clipboard`         |
| passphrase access role: event admin        | `bi-gear-fill`         |
| passphrase access role: sharable view link | `bi-share`             |


### Notifications/Alerts/Announcements
| Usage          | Icon                      |
|----------------|---------------------------|
| informative    | `bi-info-circle`          |
| description    | `bi-info-square-fill`     |
| error          | `bi-exclamation-triangle` |
| entry conflict | `bi-exclamation-diamond`  |

### Pages
| Usage                    | Icon                  |
|--------------------------|-----------------------|
| list of KüA-Plan entries | `bi-list-ul`          |
| configuration area       | `bi-gear`             |
| entry review area        | `bi-clipbaord2-check` |


## Notifications

- notifications that are shown as an immediate response to an action performed by the user SHOULD be shown as a Toast (`.toast`) in the upper right corner, using the Message Flashing mechanism from `kueaplan_server::web::ui::flash`
- all other notifications (i.e. static state of the page or errors in viewing the current page) SHOULD be shown as an alert box (`.alert`)
