from tests.conftest import ApiClientWrapper


def test_check_authorization(generated_api_client: ApiClientWrapper) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    result: kueaplan_api_client.AuthorizationInfo = generated_api_client.client.check_authorization(event_id=EVENT_ID)
    assert result.authorization == []

    generated_api_client.login(EVENT_ID, "orga")
    result = generated_api_client.client.check_authorization(event_id=EVENT_ID)
    assert len(result.authorization) == 1
    assert result.authorization[0].role == "orga"


def test_check_all_events_authorization(generated_api_client: ApiClientWrapper) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    result: kueaplan_api_client.AllEventsAuthorizationInfo = (
        generated_api_client.client.check_all_events_authorization()
    )
    assert result.events == []

    generated_api_client.login(EVENT_ID, "orga")
    result = generated_api_client.client.check_all_events_authorization()
    assert len(result.events) == 1
    assert result.events[0].event_id == EVENT_ID
    assert len(result.events[0].authorization) == 1
    assert result.events[0].authorization[0].role == "orga"

    generated_api_client.login(EVENT_ID, "user")
    result = generated_api_client.client.check_all_events_authorization()
    assert len(result.events) == 1
    assert set(auth.role for auth in result.events[0].authorization) == {"orga", "participant"}
