import datetime
import uuid

import pytest

from tests.conftest import ApiClientWrapper


def test_create_or_update_announcement(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    announcement = kueaplan_api_client.Announcement(
        id=str(uuid.uuid4()),
        announcementType="info",
        show_with_days=True,
        begin_date=datetime.date(2025, 1, 4),
        end_date=None,
        text="This is an important Announcement! I can use **Markdown** for extra highlighting.",
        sort_key=42,
    )
    generated_api_client.client.create_or_update_announcement(EVENT_ID, announcement.id, announcement)

    result = generated_api_client.client.list_announcements(EVENT_ID)
    # Set defaults, so that comparison works out
    announcement.show_with_categories = False
    announcement.categories = []
    announcement.show_with_all_categories = False
    announcement.show_with_rooms = False
    announcement.rooms = []
    announcement.show_with_all_rooms = False
    assert result[0] == announcement

    announcement.sort_key = 5
    announcement.text = "Now, the Announcement text is shorter."
    generated_api_client.client.create_or_update_announcement(EVENT_ID, announcement.id, announcement)

    result = generated_api_client.client.list_announcements(EVENT_ID)
    assert result[0] == announcement


def test_change_announcement(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    announcement = kueaplan_api_client.Announcement(
        id=str(uuid.uuid4()),
        announcementType="info",
        show_with_days=True,
        begin_date=datetime.date(2025, 1, 4),
        text="This is an important Announcement! I can use **Markdown** for extra highlighting.",
        sort_key=42,
    )
    generated_api_client.client.create_or_update_announcement(EVENT_ID, announcement.id, announcement)

    generated_api_client.client.change_announcement(
        EVENT_ID,
        announcement.id,
        generated_api_client.module.AnnouncementPatch(
            sort_key=5,
            text="Now, the Announcement text is shorter.",
        ),
    )

    result = generated_api_client.client.list_announcements(EVENT_ID)
    assert result[0].sort_key == 5
    assert result[0].show_with_days
    assert result[0].text == "Now, the Announcement text is shorter."


def test_create_or_update_announcement_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    announcement = kueaplan_api_client.Announcement(
        id=str(uuid.uuid4()),
        announcementType="info",
        text="This is an important Announcement!",
        sort_key=42,
    )
    generated_api_client.login(event_id, "user")
    # Unauthorized
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_announcement(event_id, announcement.id, announcement)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403

    generated_api_client.login(event_id, "orga")
    # Wrong id
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_announcement(event_id, str(uuid.uuid4()), announcement)
    assert "Entity id" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Non-existing event
    with pytest.raises(kueaplan_api_client.ApiException):
        generated_api_client.client.create_or_update_announcement(42, announcement.id, announcement)


def test_create_or_update_announcement_reference_errors(
    generated_api_client: ApiClientWrapper, reset_database: None
) -> None:
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

    announcement = kueaplan_api_client.Announcement(
        id=str(uuid.uuid4()),
        announcementType="info",
        text="This is an important Announcement!",
        sort_key=42,
    )

    # Non-existent room
    announcement.rooms = [test_room.id, "11111111-2222-3333-4444-555555555555"]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_announcement(event_id, announcement.id, announcement)
    assert "must reference existing rooms" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    announcement.rooms = [test_room.id]

    # Non-existent category
    announcement.categories = ["11111111-2222-3333-4444-555555555555"]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_announcement(event_id, announcement.id, announcement)
    assert "must reference existing categories" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    announcement.categories = ["019774dc-81c4-7862-a9ba-63de3d726010"]

    # Deleted room
    generated_api_client.client.delete_room(event_id, test_room.id)
    announcement.rooms = [test_room.id]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_announcement(event_id, announcement.id, announcement)
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    announcement.rooms = []

    # Deleted category
    generated_api_client.client.delete_category(event_id, test_category.id)
    announcement.categories = [test_category.id]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_announcement(event_id, announcement.id, announcement)
    assert "has been deleted" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422


def test_create_or_update_announcement_reference_errors_other_event(
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

    announcement = kueaplan_api_client.Announcement(
        id=str(uuid.uuid4()),
        announcementType="info",
        text="This is an important Announcement!",
        sort_key=42,
    )

    # room from other event
    announcement.rooms = [test_room.id]
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_announcement(event_id, announcement.id, announcement)
    assert "does not belong to event" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
    announcement.rooms = []

    # category from other event
    announcement.categories = ["019cba98-3963-7477-a04a-0ac6bfaff6bf"]  # Default category of The other event
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_announcement(event_id, announcement.id, announcement)
    assert "does not belong to event" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422
