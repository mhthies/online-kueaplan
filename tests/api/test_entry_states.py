import datetime
import uuid

import pytest

from tests.conftest import ApiClientWrapper


def test_create_draft(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")

    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        comment="wir lassen Drachen steigen",
        room=[],
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        responsible_person="Max Mustermann",
        state="draft",
        category="019774dc-81c4-7862-a9ba-63de3d726010",  # Default category from minimal.sql
        previousDates=[],
    )
    generated_api_client.client.create_or_update_entry(EVENT_ID, entry.id, entry)

    # Entry should not show up in the public list of published entries
    result = generated_api_client.client.list_entries(EVENT_ID)
    assert len(result) == 0

    # But it should show up in the list of *all* entries, accessible to orgas
    result = generated_api_client.client.list_all_entries(EVENT_ID)
    assert result[0] == entry


def test_all_entries_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "user")

    # Unauthorized
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.list_all_entries(EVENT_ID)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403
