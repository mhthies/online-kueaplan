mod sample_data;

use super::*;
use crate::web::AppState;
use actix_web::body::MessageBody;
use actix_web::{http, test, web, App};

#[actix_web::test]
async fn test_list_entries() {
    let data_store_mock = crate::data_store::store_mock::StoreMock::default();
    sample_data::fill_sample_data(&data_store_mock);
    const APP_SECRET: &str = "123456";
    let state = AppState {
        store: Arc::new(data_store_mock),
        secret: APP_SECRET.to_string(),
    };
    let app = test::init_service(
        App::new()
            .configure(configure_app)
            .app_data(web::Data::new(state.clone())),
    )
    .await;
    let mut token = SessionToken::new();
    token.add_authorization(2);
    let req = test::TestRequest::get()
        .uri("/api/v1/event/1/entries")
        .append_header(("X-SESSION-TOKEN".to_string(), token.as_string(APP_SECRET)))
        .to_request();
    let res = test::call_service(&app, req).await;
    let res_status = res.status();
    println!("{:#?}", res);
    let body = res.into_body().try_into_bytes().unwrap();
    println!("{:#?}", body);
    assert_eq!(res_status, http::StatusCode::OK);
    let result: Vec<kueaplan_api_types::Entry> = serde_json::from_slice(&body).unwrap();
    assert!(!result.is_empty());
}
