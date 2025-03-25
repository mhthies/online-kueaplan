use crate::auth_session::SessionToken;
use crate::data_store::models::{FullNewEntry, NewCategory, NewEntry, NewEvent};
use crate::data_store::{GlobalAuthToken, KuaPlanStore};
use chrono::TimeZone;
use uuid::uuid;

pub(crate) fn fill_sample_data(store: &impl KuaPlanStore) {
    let mut facade = store.get_facade().unwrap();
    let cli_auth_token = crate::CliAuthToken::new();
    let admin_token = GlobalAuthToken::get_global_cli_authorization(&cli_auth_token);
    facade
        .create_event(
            &admin_token,
            NewEvent {
                title: "SommerAkademie 2024".to_string(),
                begin_date: chrono::NaiveDate::from_ymd_opt(2024, 7, 27).unwrap(),
                end_date: chrono::NaiveDate::from_ymd_opt(2024, 8, 10).unwrap(),
            },
        )
        .unwrap();
    let mut session_token = SessionToken::new();
    session_token.add_authorization(2);
    let auth_token = facade.check_authorization(&session_token, 42).unwrap();
    facade
        .create_or_update_category(
            &auth_token,
            NewCategory {
                id: uuid!("019586d4-08fa-7341-9bee-d223c46e77cc"),
                title: "Default Category".to_string(),
                icon: "".to_string(),
                color: "000000".to_string(),
                event_id: 42,
                is_official: false,
            },
        )
        .unwrap();
    facade.create_or_update_entry(&auth_token, FullNewEntry{
        entry: NewEntry {
            id: uuid!("fca6379a-b8ad-4a53-9479-73099c34f16a"),
            title: "Kennenlernspiele".to_string(),
            description: "Es sollen heute wieder Kennenlernspiele stattfinden.
Sie starten sobald die KL- und Minderjährigentreffen vorbei sind in der Pelikanhalle.
Es wird auch wieder für Leute, die es ruhiger mögen einen zweiten ruhigeren Ort geben.

Die Spiele sind dafür da, neue Leute kennenzulernen, Menschen mit ähnlichen Interessen und Anschluss auf der Akademie zu finden.
Deswegen richten sie sich explizit sowohl an Menschen, die jetzt neu dazugekommen sind, als auch die, die schon lange im Verein sind.

Treffpunkt: Pelikanhalle".to_string(),
            responsible_person: "Sam, Amity".to_string(),
            is_room_reservation: false,
            event_id: 42,
            begin: chrono::Utc.with_ymd_and_hms(2024, 7, 27, 21, 45, 0).unwrap(),
            end: chrono::Utc.with_ymd_and_hms(2024, 7, 27, 23, 0, 0).unwrap(),
            category: uuid!("019586d4-08fa-7341-9bee-d223c46e77cc"),
            comment: "".to_string(),
            room_comment: "".to_string(),
            time_comment: "".to_string(),
            is_exclusive: false,
            is_cancelled: false,
        },
        room_ids: vec![],
        previous_dates: vec![],
    }, false).unwrap();
}
