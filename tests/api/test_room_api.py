import datetime
import uuid

import pytest

from tests.conftest import ApiClientWrapper


def test_create_or_update_room(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
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


def test_create_or_update_room_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="This is the unofficial room, deep down underground.",
    )
    generated_api_client.login(event_id, "user")
    # Unauthorized
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


def test_delete_room(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(EVENT_ID, room.id, room)

    result = generated_api_client.client.list_rooms(EVENT_ID)
    assert len(result) == 1

    generated_api_client.client.delete_room(EVENT_ID, room.id)

    result = generated_api_client.client.list_rooms(EVENT_ID)
    assert len(result) == 0


def test_delete_room_with_replacement(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")
    # given: two rooms and an entry, which takes place in one room and has a previous date in this room
    room1 = kueaplan_api_client.Room(id=str(uuid.uuid4()), title="Test Room", description="")
    room2 = kueaplan_api_client.Room(id=str(uuid.uuid4()), title="Test Room 2", description="")
    generated_api_client.client.create_or_update_room(event_id, room1.id, room1)
    generated_api_client.client.create_or_update_room(event_id, room2.id, room2)
    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        room=[room1.id],
        room_comment="Das macht Spaß",
        responsible_person="Max Mustermann",
        category="019774dc-81c4-7862-a9ba-63de3d726010",  # Default category from database dump
        previous_dates=[
            kueaplan_api_client.PreviousDate(
                id=str(uuid.uuid4()),
                begin=datetime.datetime(2025, 1, 6, 13, 0, tzinfo=datetime.UTC).isoformat(),
                end=datetime.datetime(2025, 1, 6, 14, 30, tzinfo=datetime.UTC).isoformat(),
                room=[room1.id],
            )
        ],
    )
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    # when: we delete the room and specify the second room as replacement (as well as an additional room comment)
    generated_api_client.client.delete_room(
        event_id,
        room1.id,
        kueaplan_api_client.DeleteRoomRequest(replace_rooms=[room2.id], add_room_comment="war im alten Testraum"),
    )

    # and then: the entry and its previous date are assigned to the second room (and the room_comment is amended)
    new_entry = generated_api_client.client.get_entry(event_id, entry.id)
    assert new_entry.room == [room2.id]
    assert new_entry.previous_dates[0].room == [room2.id]
    assert new_entry.room_comment == "Das macht Spaß; war im alten Testraum"


def test_delete_room_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")

    # Non-existing room
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_room(event_id, "11111111-2222-3333-4444-555555555555")
    assert "not exist" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 404

    # Create a new room and entry for testing
    room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(event_id, room.id, room)
    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        room=[room.id],
        room_comment="Das macht Spaß",
        responsible_person="Max Mustermann",
        category="019774dc-81c4-7862-a9ba-63de3d726010",  # Default category from database dump
        previous_dates=[],
    )
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    # Non-existing replacement
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_room(
            event_id,
            room.id,
            kueaplan_api_client.DeleteRoomRequest(replace_rooms=["11111111-2222-3333-4444-555555555555"]),
        )
    assert "must reference existing rooms" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409

    # Room replaced with itself
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_room(
            event_id, room.id, kueaplan_api_client.DeleteRoomRequest(replace_rooms=[room.id])
        )
    assert excinfo.value.data.http_code == 409

    # Unauthorized
    del generated_api_client.client.api_client.configuration.api_key["sessionTokenAuth"]
    generated_api_client.login(event_id, "user")
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_room(event_id, room.id)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403


def test_room_id_conflicts(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    other_event_id = 2

    generated_api_client.login(event_id, "orga")
    generated_api_client.login(other_event_id, "orga")

    room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(event_id, room.id, room)

    # Same room in other event
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_room(other_event_id, room.id, room)
    assert "already exists" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409

    # room has been deleted
    generated_api_client.client.delete_room(event_id, room.id)
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_room(other_event_id, room.id, room)
    assert "already exists" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409
