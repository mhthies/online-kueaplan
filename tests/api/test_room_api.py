import uuid

import pytest

from tests.conftest import ApiClientWrapper


def test_create_or_update_room(generated_api_client: ApiClientWrapper) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="This is the unofficial room, deep down underground.",
    )
    generated_api_client.client.create_or_update_room(EVENT_ID, room.id, room)

    result = generated_api_client.client.list_rooms(EVENT_ID)
    # Categories are ordered by sort_key. Default room is 0, so our room comes second
    assert result[0] == room

    room.title = "Secret Room"
    generated_api_client.client.create_or_update_room(EVENT_ID, room.id, room)

    result = generated_api_client.client.list_rooms(EVENT_ID)
    assert result[0] == room


def test_create_or_update_room_errors(generated_api_client: ApiClientWrapper) -> None:
    import kueaplan_api_client

    event_id = 1
    room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="This is the unofficial room, deep down underground.",
    )
    generated_api_client.login(event_id, "user")
    # Unauthenticated
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_room(event_id, room.id, room)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403

    generated_api_client.login(event_id, "orga")
    # Wrong id
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_room(event_id, str(uuid.uuid4()), room)
    assert "Entity id" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Non-existing event
    with pytest.raises(kueaplan_api_client.ApiException):
        generated_api_client.client.create_or_update_room(42, room.id, room)
