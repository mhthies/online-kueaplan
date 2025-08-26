import uuid
from types import ModuleType

import pytest

from tests.conftest import ApiClientWrapper


def test_create_or_update_announcement(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    announcement: "kuaeplan_api_client.Announcement" = generated_api_client.module.Announcement(
        id=str(uuid.uuid4()),
        announcementType="info",
        text="This is an important Announcement! I can use **Markdown** for extra highlighting.",
        sortKey=42,
    )
    generated_api_client.client.create_or_update_announcement(EVENT_ID, announcement.id, announcement)

    result = generated_api_client.client.list_announcements(EVENT_ID)
    assert result[0] == announcement

    announcement.sortKey = 5
    announcement.text = "Now, the Announcement text is shorter."
    generated_api_client.client.create_or_update_announcement(EVENT_ID, announcement.id, announcement)

    result = generated_api_client.client.list_announcements(EVENT_ID)
    assert result[0] == announcement

def test_create_or_update_announcement_errors(generated_api_client: ApiClientWrapper) -> None:
    event_id = 1
    announcement: "kuaeplan_api_client.Announcement" = generated_api_client.module.Announcement(
        id=str(uuid.uuid4()),
        announcementType="info",
        text="This is an important Announcement!",
        sortKey=42,
    )
    generated_api_client.login(event_id, "user")
    # Unauthenticated
    with pytest.raises(Exception) as excinfo:
        generated_api_client.client.create_or_update_announcement(event_id, announcement.id, announcement)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403

    generated_api_client.login(event_id, "orga")
    # Wrong id
    with pytest.raises(Exception) as excinfo:
        generated_api_client.client.create_or_update_announcement(event_id, str(uuid.uuid4()), announcement)
    assert "Entity id" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Non-existing event
    with pytest.raises(Exception):
        generated_api_client.client.create_or_update_announcement(42, announcement.id, announcement)
