import datetime
import uuid

import pytest

from tests.conftest import ApiClientWrapper

# TODO test retrieving filtered list of events


def test_create_and_update_entry_simple(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    test_room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(EVENT_ID, test_room.id, test_room)

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
    entry.is_cancelled = None
    entry.is_exclusive = True
    entry.time_comment = None
    entry.room = [test_room.id]
    entry.room_comment = "Im Testraum unten"
    entry.begin = datetime.datetime(2025, 1, 6, 12, 5, tzinfo=datetime.UTC).isoformat()
    entry.end = datetime.datetime(2025, 1, 6, 13, 30, 45, tzinfo=datetime.UTC).isoformat()
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
    # Unauthorized
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
    entry.room = [test_room.id]

    # Non-existent category
    entry.category = "11111111-2222-3333-4444-555555555555"
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "must reference an existing category" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    entry.category = "019774dc-81c4-7862-a9ba-63de3d726010"

    # Non-existent room in previous date
    entry.previous_dates = [
        kueaplan_api_client.PreviousDate(
            id=str(uuid.uuid4()),
            begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
            end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
            room=[test_room.id, "11111111-2222-3333-4444-555555555555"],
        )
    ]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "must reference existing rooms" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    entry.previous_dates = []

    # Deleted room
    generated_api_client.client.delete_room(event_id, test_room.id)
    entry.room = [test_room.id]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    entry.room = []

    # Deleted room in previous date
    entry.previous_dates = [
        kueaplan_api_client.PreviousDate(id=str(uuid.uuid4()), begin=entry.begin, end=entry.end, room=[test_room.id])
    ]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Deleted category
    generated_api_client.client.delete_category(event_id, test_category.id)
    entry.category = test_category.id
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422


def test_create_or_update_entry_reference_errors_other_event(
    generated_api_client: ApiClientWrapper, reset_database: None
) -> None:
    import kueaplan_api_client

    event_id = 1
    other_event_id = 2

    generated_api_client.login(event_id, "orga")
    generated_api_client.login(other_event_id, "orga")
    test_room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(other_event_id, test_room.id, test_room)

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

    # room from other event
    entry.room = [test_room.id]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "does not belong to event" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    entry.room = []

    # room from other event in previous date
    entry.previous_dates = [
        kueaplan_api_client.PreviousDate(
            id=str(uuid.uuid4()),
            room=[test_room.id],
            begin=entry.begin,
            end=entry.end,
            comment="",
        )
    ]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "does not belong to event" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    entry.previous_dates = []

    # category from other event
    entry.category = "019cba98-3963-7477-a04a-0ac6bfaff6bf"  # Default category of The other event
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    assert "does not belong to event" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422


def test_change_entry_simple(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")
    test_room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(event_id, test_room.id, test_room)

    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        comment="wir lassen Drachen steigen",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        time_comment="direkt nach dem Mittagessen",
        responsible_person="Max Mustermann",
        is_cancelled=True,
        category="019774dc-81c4-7862-a9ba-63de3d726010",  # Default category from minimal.sql
        previousDates=[],
    )
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    result = generated_api_client.client.list_entries(event_id)
    # Categories are ordered by sort_key. Default room is 0, so our room comes second
    assert result[0] == entry

    entry.title = "Drachenfliegen für jedermann"
    entry.is_cancelled = None
    entry.is_exclusive = True
    entry.time_comment = None
    entry.room = [test_room.id]
    entry.room_comment = "Im Testraum unten"
    entry.begin = datetime.datetime(2025, 1, 6, 12, 5, tzinfo=datetime.UTC).isoformat()
    entry.end = datetime.datetime(2025, 1, 6, 13, 30, 45, tzinfo=datetime.UTC).isoformat()

    generated_api_client.client.change_entry(
        event_id,
        entry.id,
        kueaplan_api_client.EntryPatch(
            title="Drachenfliegen für jedermann",
            is_cancelled=False,
            is_exclusive=True,
            time_comment="",
            room=[test_room.id],
            room_comment="Im Testraum unten",
            begin=datetime.datetime(2025, 1, 6, 12, 5, tzinfo=datetime.UTC).isoformat(),
            end=datetime.datetime(2025, 1, 6, 13, 30, 45, tzinfo=datetime.UTC).isoformat(),
        ),
    )

    result = generated_api_client.client.get_entry(event_id, entry.id)
    assert result == entry


def test_change_entry_simple_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    # First, login as orga to create the entry, but delete the session token afterward, to drop privileges
    generated_api_client.login(event_id, "orga")
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
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    del generated_api_client.client.api_client.configuration.api_key["sessionTokenAuth"]

    generated_api_client.login(event_id, "user")
    # Unauthorized
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.change_entry(
            event_id, entry.id, kueaplan_api_client.EntryPatch(title="Drachenfliegen für jedermann")
        )
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403

    # Non-existing entry
    generated_api_client.login(event_id, "orga")
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.change_entry(
            event_id,
            "11111111-2222-3333-4444-555555555555",
            kueaplan_api_client.EntryPatch(title="Drachenfliegen für jedermann"),
        )
    assert "not exist" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 404


def test_patch_reference_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
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
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    # Non-existent room
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.change_entry(
            event_id,
            entry.id,
            kueaplan_api_client.EntryPatch(room=[test_room.id, "11111111-2222-3333-4444-555555555555"]),
        )
    assert "must reference existing rooms" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Non-existent category
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.change_entry(
            event_id, entry.id, kueaplan_api_client.EntryPatch(category="11111111-2222-3333-4444-555555555555")
        )
    assert "must reference an existing category" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Deleted room
    generated_api_client.client.delete_room(event_id, test_room.id)
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.change_entry(
            event_id, entry.id, kueaplan_api_client.EntryPatch(room=[test_room.id])
        )
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Deleted category
    generated_api_client.client.delete_category(event_id, test_category.id)
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.change_entry(
            event_id, entry.id, kueaplan_api_client.EntryPatch(category=test_category.id)
        )
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422


def test_change_entry_reference_errors_other_event(
    generated_api_client: ApiClientWrapper, reset_database: None
) -> None:
    import kueaplan_api_client

    event_id = 1
    other_event_id = 2

    generated_api_client.login(event_id, "orga")
    generated_api_client.login(other_event_id, "orga")
    test_room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(other_event_id, test_room.id, test_room)

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
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    # room from other event
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.change_entry(
            event_id, entry.id, kueaplan_api_client.EntryPatch(room=[test_room.id])
        )
    assert "does not belong to event" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # category from other event
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.change_entry(
            event_id,
            entry.id,
            kueaplan_api_client.EntryPatch(
                category="019cba98-3963-7477-a04a-0ac6bfaff6bf"  # Default category of The other event
            ),
        )
    assert "does not belong to event" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422


def test_create_and_update_previous_date(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    test_room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(EVENT_ID, test_room.id, test_room)

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
    generated_api_client.client.create_or_update_entry(EVENT_ID, entry.id, entry)

    previous_date = kueaplan_api_client.PreviousDate(
        id=str(uuid.uuid4()),
        begin=datetime.datetime(2025, 1, 6, 12, 15, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 45, tzinfo=datetime.UTC).isoformat(),
        room=[test_room.id],
        comment="Verschoben wegen Raumkonflikt",
    )
    generated_api_client.client.create_or_update_previous_date(EVENT_ID, entry.id, previous_date.id, previous_date)

    result = generated_api_client.client.get_entry(EVENT_ID, entry.id)
    assert result.previous_dates[0] == previous_date

    previous_date.room = []
    previous_date.comment = "Verschoben, wegen quengelnder Kursleiter*innen"
    generated_api_client.client.create_or_update_previous_date(EVENT_ID, entry.id, previous_date.id, previous_date)

    result = generated_api_client.client.get_entry(EVENT_ID, entry.id)
    assert result.previous_dates[0] == previous_date


def test_create_or_update_previous_date_simple_errors(
    generated_api_client: ApiClientWrapper, reset_database: None
) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")
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
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    previous_date = kueaplan_api_client.PreviousDate(
        id=str(uuid.uuid4()),
        begin=datetime.datetime(2025, 1, 6, 12, 15, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 45, tzinfo=datetime.UTC).isoformat(),
        room=[],
        comment="Verschoben wegen Raumkonflikt",
    )

    # Unauthorized
    del generated_api_client.client.api_client.configuration.api_key["sessionTokenAuth"]
    generated_api_client.login(event_id, "user")
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_previous_date(event_id, entry.id, previous_date.id, previous_date)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403

    generated_api_client.login(event_id, "orga")
    # Wrong id
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_previous_date(event_id, entry.id, str(uuid.uuid4()), previous_date)
    assert "Entity id" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Non-existing entry
    with pytest.raises(kueaplan_api_client.ApiException):
        generated_api_client.client.create_or_update_previous_date(42, entry.id, previous_date.id, previous_date)

    # TODO conflict with previous_date id within another entry


def test_create_or_update_previous_date_reference_errors(
    generated_api_client: ApiClientWrapper, reset_database: None
) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")
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
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)
    previous_date = kueaplan_api_client.PreviousDate(
        id=str(uuid.uuid4()),
        begin=datetime.datetime(2025, 1, 6, 12, 15, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 45, tzinfo=datetime.UTC).isoformat(),
        room=[],
        comment="Verschoben wegen Raumkonflikt",
    )

    # non-existing room
    previous_date.room = ["11111111-2222-3333-4444-555555555555"]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_previous_date(event_id, entry.id, previous_date.id, previous_date)
    assert "must reference existing rooms" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # deleted room
    generated_api_client.client.delete_room(event_id, test_room.id)
    previous_date.room = [test_room.id]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_previous_date(event_id, entry.id, previous_date.id, previous_date)
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # room from another event
    other_event_id = 2
    generated_api_client.login(other_event_id, "orga")
    other_test_room = kueaplan_api_client.Room(
        id=str(uuid.uuid4()),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(other_event_id, other_test_room.id, other_test_room)
    previous_date.room = [other_test_room.id]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_previous_date(event_id, entry.id, previous_date.id, previous_date)
    assert "does not belong to event" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422


def test_delete_previous_date(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")
    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        responsible_person="Max Mustermann",
        category="019774dc-81c4-7862-a9ba-63de3d726010",  # Default category from minimal.sql
        previousDates=[
            kueaplan_api_client.PreviousDate(
                id=str(uuid.uuid4()),
                begin=datetime.datetime(2025, 1, 6, 12, 15, tzinfo=datetime.UTC).isoformat(),
                end=datetime.datetime(2025, 1, 6, 13, 45, tzinfo=datetime.UTC).isoformat(),
                room=[],
                comment="Verschoben wegen Raumkonflikt",
            )
        ],
    )
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    generated_api_client.client.delete_previous_date(event_id, entry.id, entry.previous_dates[0].id)

    result = generated_api_client.client.get_entry(event_id, entry.id)
    assert len(result.previous_dates) == 0


def test_delete_previous_date_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")
    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        responsible_person="Max Mustermann",
        category="019774dc-81c4-7862-a9ba-63de3d726010",  # Default category from minimal.sql
        previousDates=[
            kueaplan_api_client.PreviousDate(
                id=str(uuid.uuid4()),
                begin=datetime.datetime(2025, 1, 6, 12, 15, tzinfo=datetime.UTC).isoformat(),
                end=datetime.datetime(2025, 1, 6, 13, 45, tzinfo=datetime.UTC).isoformat(),
                room=[],
                comment="Verschoben wegen Raumkonflikt",
            )
        ],
    )
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    # Non-existing/wrong entry
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_previous_date(
            event_id, "11111111-2222-3333-4444-555555555555", entry.previous_dates[0].id
        )
    assert "not exist" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 404

    # Unauthorized
    del generated_api_client.client.api_client.configuration.api_key["sessionTokenAuth"]
    generated_api_client.login(event_id, "user")
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_previous_date(event_id, entry.id, entry.previous_dates[0].id)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403


def test_delete_entry(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")
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
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    result = generated_api_client.client.list_entries(event_id)
    assert len(result) == 1

    generated_api_client.client.delete_entry(event_id, entry.id)

    result = generated_api_client.client.list_entries(event_id)
    assert len(result) == 0

    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.get_entry(event_id, entry.id)
    assert "not exist" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 404


def test_delete_entry_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")
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
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    # Non-existing entry
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_entry(event_id, "11111111-2222-3333-4444-555555555555")
    assert "not exist" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 404

    # Unauthorized
    del generated_api_client.client.api_client.configuration.api_key["sessionTokenAuth"]
    generated_api_client.login(event_id, "user")
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_entry(event_id, entry.id)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403
