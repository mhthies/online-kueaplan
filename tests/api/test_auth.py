import uuid

import pytest

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


def test_drop_access_role(generated_api_client: ApiClientWrapper) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    generated_api_client.login(EVENT_ID, "user")
    result = generated_api_client.client.check_authorization(event_id=EVENT_ID)
    assert set(auth.role for auth in result.authorization) == {"orga", "participant"}

    result = generated_api_client.client.drop_access_role(
        EVENT_ID, kueaplan_api_client.DropAccessRoleRequest(role="orga")
    )
    generated_api_client.client.api_client.configuration.api_key["sessionTokenAuth"] = result.session_token
    assert set(auth.role for auth in result.authorization) == {"participant"}

    # participant should allow to fetch entries
    generated_api_client.client.list_entries(event_id=EVENT_ID)
    # ... but trying to add create a room should fail
    room_id = uuid.uuid4()
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_room(
            event_id=EVENT_ID, room_id=room_id, room=kueaplan_api_client.Room(id=room_id, title="Test", description="")
        )
    assert excinfo.value.status == 403
