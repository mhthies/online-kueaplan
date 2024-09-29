use super::*;
use actix_web::{http, test, web, App};
use actix_web::body::MessageBody;

#[actix_web::test]
async fn test_list_entries() {
    let data_store_mock = Arc::new(crate::data_store::store_mock::StoreMock::default());
    const APP_SECRET: &str = "123456";
    // TODO insert test entries
    let state = AppState {
        store: data_store_mock.clone(),
        secret: APP_SECRET.to_string(),
    };
    let app = test::init_service(
        App::new()
            .configure(configure_app)
            .app_data(web::Data::new(state.clone())),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/api/v1/event/1/entries")
        .append_header((
            "X-SESSION-TOKEN".to_string(),
            crate::auth_session::SessionToken::new().as_string(APP_SECRET),
        ))
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
