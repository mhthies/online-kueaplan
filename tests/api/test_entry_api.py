import datetime
import uuid

import pytest

from tests.conftest import ApiClientWrapper

# TODO test retrieving filtered list of events


def test_create_and_update_entry_simple(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        comment="wir lassen Drachen steigen",
        description="""Wir bauen Drachen und lassen sie steigen.

        Für das Material müssen von jedem Teilnehmer an der KüA **5€** bezahlt werden.
        """,
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        time_comment="direkt nach dem Mittagessen",
        responsible_person="Max Mustermann",
        is_cancelled=True,
        category="019774dc-81c4-7862-a9ba-63de3d726010",  # Default category from minimal.sql
        previousDates=[],
    )
    generated_api_client.client.create_or_update_entry(EVENT_ID, entry.id, entry)

    result = generated_api_client.client.list_entries(EVENT_ID)
    # Categories are ordered by sort_key. Default room is 0, so our room comes second
    assert result[0] == entry

    entry.title = "Drachenfliegen für jedermann"
    # TODO change more attributes
    entry.is_cancelled = None
    generated_api_client.client.create_or_update_entry(EVENT_ID, entry.id, entry)

    result = generated_api_client.client.get_entry(EVENT_ID, entry.id)
    assert result == entry


def test_create_or_update_entry_simple_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        responsible_person="Max Mustermann",
        category="019774dc-81c4-7862-a9ba-63de3d726010",  # Default category from minimal.sql
        previousDates=[],
    )
    generated_api_client.login(event_id, "user")
    # Unauthenticated
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403

    generated_api_client.login(event_id, "orga")
    # Wrong id
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, str(uuid.uuid4()), entry)
    assert "Entity id" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Non-existing event
    with pytest.raises(kueaplan_api_client.ApiException):
        generated_api_client.client.create_or_update_entry(42, entry.id, entry)


def test_create_or_update_entry_reference_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")
    test_category = kueaplan_api_client.Category(
        id=str(uuid.uuid4()),
        title="Test Category",
        icon="💡",
        color="ffaa00",
        sort_key=42,
    )
    generated_api_client.client.create_or_update_category(event_id, test_category.id, test_category)
    test_room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(event_id, test_room.id, test_room)

    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        responsible_person="Max Mustermann",
        category="019774dc-81c4-7862-a9ba-63de3d726010",  # Default category from minimal.sql
        previousDates=[],
    )

    # Non-existent room
    entry.room = [test_room.id, "11111111-2222-3333-4444-555555555555"]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "must reference existing rooms" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Non-existent category
    entry.room = [test_room.id]
    entry.category = "11111111-2222-3333-4444-555555555555"
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "must reference an existing category" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Deleted room
    generated_api_client.client.delete_room(event_id, test_room.id)
    entry.room = [test_room.id]
    entry.category = "019774dc-81c4-7862-a9ba-63de3d726010"
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Deleted category
    generated_api_client.client.delete_category(event_id, test_category.id)
    entry.room = []
    entry.category = test_category.id
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # TODO previous date with non-existent or deleted room


# TODO test for reference errors due to room/category from wrong event
# TODO create other event via psql command (using command-line interface is to complicated)

# TODO test patching event

# TODO test for errors while patching event (same as test_create_or_update_entry_simple_errors() and
#   test_create_or_update_entry_reference_errors()

# TODO test deleting event

# TODO test for errors while deleting event
