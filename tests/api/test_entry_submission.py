import datetime
import uuid

import pytest

from tests.conftest import ApiClientWrapper


def test_entry_submission_with_prior_review(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "admin")
    _set_entry_submission_mode(generated_api_client, 1, "review-before-publishing")

    generated_api_client.clear_login()
    generated_api_client.login(event_id, "user")
    entry = kueaplan_api_client.EntrySubmission(
        id=uuid.uuid4(),
        title="Drachenfliegen leicht gemacht",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC),
        responsiblePerson="Max Mustermann",
        category=uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010"),  # Default category from minimal.sql
    )
    generated_api_client.client.submit_entry(event_id, entry)

    public_entries = generated_api_client.client.list_entries(event_id)
    assert public_entries == []

    generated_api_client.login(event_id, "orga")
    result = generated_api_client.client.list_all_entries(event_id)
    assert len(result) == 1
    assert result[0].id == entry.id
    assert result[0].title == entry.title
    assert result[0].begin == entry.begin
    assert result[0].end == entry.end
    assert result[0].responsible_person == entry.responsible_person
    assert result[0].category == entry.category
    assert result[0].state == "submitted-for-review"


def test_entry_submission_without_prior_review(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "admin")
    _set_entry_submission_mode(generated_api_client, 1, "review-after-publishing")

    generated_api_client.clear_login()
    generated_api_client.login(event_id, "user")
    entry = kueaplan_api_client.EntrySubmission(
        id=uuid.uuid4(),
        title="Drachenfliegen leicht gemacht",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC),
        responsiblePerson="Max Mustermann",
        category=uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010"),  # Default category from minimal.sql
        publishWithoutReview=True,
    )
    generated_api_client.client.submit_entry(event_id, entry)

    result = generated_api_client.client.list_entries(event_id)
    assert len(result) == 1
    assert result[0].id == entry.id
    assert result[0].title == entry.title
    assert result[0].begin == entry.begin
    assert result[0].end == entry.end
    assert result[0].responsible_person == entry.responsible_person
    assert result[0].category == entry.category
    assert result[0].state == "preliminary-published"


def test_entry_submission_auth_error(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    entry = kueaplan_api_client.EntrySubmission(
        id=uuid.uuid4(),
        title="Drachenfliegen leicht gemacht",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC),
        responsiblePerson="Max Mustermann",
        category=uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010"),  # Default category from minimal.sql
    )

    # Unauthorized
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "requires authentication" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403


def test_entry_submission_mode_error(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "user")
    entry = kueaplan_api_client.EntrySubmission(
        id=uuid.uuid4(),
        title="Drachenfliegen leicht gemacht",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC),
        responsiblePerson="Max Mustermann",
        category=uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010"),  # Default category from minimal.sql
    )

    # Entry submission by participants is not enabled
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "Entry submission must be enabled." in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409

    generated_api_client.login(event_id, "admin")
    _set_entry_submission_mode(generated_api_client, 1, "review-before-publishing")

    generated_api_client.clear_login()
    generated_api_client.login(event_id, "user")

    # Entry submission by participants without prior review is not enabled
    entry.publish_without_review = True
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "a review state, allowed by the entry submission mode." in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409


def test_entry_submission_conflict(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "admin")
    _set_entry_submission_mode(generated_api_client, 1, "review-before-publishing")

    test_room = kueaplan_api_client.Room(
        id=uuid.uuid4(),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(event_id, test_room.id, test_room)

    room_conflict_entry = kueaplan_api_client.Entry(
        id=uuid.uuid4(),
        title="Test-Raum gesperrt für geheime Dinge",
        room=[test_room.id],
        begin=datetime.datetime(2025, 1, 6, 13, 0, tzinfo=datetime.UTC),
        end=datetime.datetime(2025, 1, 6, 14, 0, tzinfo=datetime.UTC),
        responsiblePerson="Die Orgas",
        isRoomReservation=True,
        category=uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010"),  # Default category from minimal.sql
        previousDates=[],
    )
    generated_api_client.client.create_or_update_entry(event_id, room_conflict_entry.id, room_conflict_entry)
    exclusive_entry = kueaplan_api_client.Entry(
        id=uuid.uuid4(),
        title="Plenum",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 19, 0, tzinfo=datetime.UTC),
        end=datetime.datetime(2025, 1, 6, 20, 0, tzinfo=datetime.UTC),
        responsiblePerson="Die Orgas",
        isExclusive=True,
        category=uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010"),  # Default category from minimal.sql
        previousDates=[],
    )
    generated_api_client.client.create_or_update_entry(event_id, exclusive_entry.id, exclusive_entry)

    generated_api_client.clear_login()
    generated_api_client.login(event_id, "user")

    entry = kueaplan_api_client.EntrySubmission(
        id=uuid.uuid4(),
        title="Drachenfliegen leicht gemacht",
        room=[test_room.id],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC),
        responsiblePerson="Max Mustermann",
        category=uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010"),  # Default category from minimal.sql
    )

    # Entry conflicts with other entry in same room
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "Submitted entry must not cause a room conflict" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409

    # Entry conflicts with exclusive entry
    entry.room = []
    entry.begin = datetime.datetime(2025, 1, 6, 18, 30, tzinfo=datetime.UTC)
    entry.end = datetime.datetime(2025, 1, 6, 19, 15, tzinfo=datetime.UTC)
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "Submitted entry must not cause a conflict with an exclusive entry." in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409


def test_entry_submission_reference_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "admin")
    _set_entry_submission_mode(generated_api_client, event_id, "review-before-publishing")
    test_category = kueaplan_api_client.Category(
        id=uuid.uuid4(),
        title="Test Category",
        icon="💡",
        color="ffaa00",
        sort_key=42,
    )
    generated_api_client.client.create_or_update_category(event_id, test_category.id, test_category)
    test_room = kueaplan_api_client.Room(
        id=uuid.uuid4(),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(event_id, test_room.id, test_room)

    entry = kueaplan_api_client.EntrySubmission(
        id=uuid.uuid4(),
        title="Drachenfliegen leicht gemacht",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC),
        responsiblePerson="Max Mustermann",
        category=uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010"),  # Default category from minimal.sql
    )

    # Non-existent room
    entry.room = [test_room.id, uuid.UUID("11111111-2222-3333-4444-555555555555")]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "must reference existing rooms" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    entry.room = [test_room.id]

    # Non-existent category
    entry.category = uuid.UUID("11111111-2222-3333-4444-555555555555")
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "must reference an existing category" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    entry.category = uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010")

    # Deleted room
    generated_api_client.client.delete_room(event_id, test_room.id)
    entry.room = [test_room.id]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    entry.room = []

    # Deleted category
    generated_api_client.client.delete_category(event_id, test_category.id)
    entry.category = test_category.id
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422


def test_entry_submission_reference_errors_other_event(
    generated_api_client: ApiClientWrapper, reset_database: None
) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "admin")
    _set_entry_submission_mode(generated_api_client, event_id, "review-before-publishing")
    other_event_id = 2

    generated_api_client.login(event_id, "orga")
    generated_api_client.login(other_event_id, "orga")
    test_room = kueaplan_api_client.Room(
        id=uuid.uuid4(),
        title="Test Room",
        description="",
    )
    generated_api_client.client.create_or_update_room(other_event_id, test_room.id, test_room)

    entry = kueaplan_api_client.EntrySubmission(
        id=uuid.uuid4(),
        title="Drachenfliegen leicht gemacht",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC),
        responsiblePerson="Max Mustermann",
        category=uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010"),  # Default category from minimal.sql
    )

    # room from other event
    entry.room = [test_room.id]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "does not belong to event" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    entry.room = []

    # category from other event
    entry.category = uuid.UUID("019cba98-3963-7477-a04a-0ac6bfaff6bf")  # Default category of The other event
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.submit_entry(event_id, entry)
    assert "does not belong to event" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422


def _set_entry_submission_mode(client_wrapper: ApiClientWrapper, event_id: int, mode: str) -> None:
    event_info = client_wrapper.client.get_extended_event_info(event_id)
    event_info.entry_submission_mode = mode
    client_wrapper.client.update_extended_event(event_id, event_info)
