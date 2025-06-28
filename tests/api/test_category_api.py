import uuid
from types import ModuleType

import pytest

from tests.conftest import ApiClientWrapper


def test_list_categories(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "user")
    result = generated_api_client.client.list_categories(EVENT_ID)
    assert len(result) == 1
    assert result[0].title == "Default"

def test_create_or_update_category(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    category: "kuaeplan_api_client.Category" = generated_api_client.module.Category(
        id=str(uuid.uuid4()),
        title="Test Category",
        icon="💡",
        color="ffaa00",
        sort_key=42,
    )
    generated_api_client.client.create_or_update_category(EVENT_ID, category.id, category)

    result = generated_api_client.client.list_categories(EVENT_ID)
    # Categories are ordered by sort_key. Default category is 0, so our category comes second
    assert result[1] == category

    category.icon = "🙂"
    category.title = "Test"
    generated_api_client.client.create_or_update_category(EVENT_ID, category.id, category)

    result = generated_api_client.client.list_categories(EVENT_ID)
    assert result[1] == category

def test_create_or_update_category_errors(generated_api_client: ApiClientWrapper) -> None:
    event_id = 1
    category: "kuaeplan_api_client.Category" = generated_api_client.module.Category(
        id=str(uuid.uuid4()),
        title="Test Category",
        icon="💡",
        color="ffaa00",
        sort_key=42,
    )
    generated_api_client.login(event_id, "user")
    # Unauthenticated
    with pytest.raises(Exception) as excinfo:
        generated_api_client.client.create_or_update_category(event_id, category.id, category)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403

    generated_api_client.login(event_id, "orga")
    # Wrong id
    with pytest.raises(Exception) as excinfo:
        generated_api_client.client.create_or_update_category(event_id, str(uuid.uuid4()), category)
    assert "Entity id" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Non-existing event
    with pytest.raises(Exception):
        generated_api_client.client.create_or_update_category(42, category.id, category)
