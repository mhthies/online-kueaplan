import datetime
import uuid

import pytest

from tests.conftest import ApiClientWrapper


def test_list_categories(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "user")
    result = generated_api_client.client.list_categories(EVENT_ID)
    assert len(result) == 1
    assert result[0].title == "Default"


def test_create_or_update_category(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    category = kueaplan_api_client.Category(
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


def test_create_or_update_category_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    category = kueaplan_api_client.Category(
        id=str(uuid.uuid4()),
        title="Test Category",
        icon="💡",
        color="ffaa00",
        sort_key=42,
    )
    generated_api_client.login(event_id, "user")
    # Unauthorized
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_category(event_id, category.id, category)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403

    generated_api_client.login(event_id, "orga")
    # Wrong id
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_category(event_id, str(uuid.uuid4()), category)
    assert "Entity id" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 422

    # Non-existing event
    with pytest.raises(kueaplan_api_client.ApiException):
        generated_api_client.client.create_or_update_category(42, category.id, category)


def test_delete_category(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "orga")
    category = kueaplan_api_client.Category(
        id=str(uuid.uuid4()),
        title="Test Category",
        icon="💡",
        color="ffaa00",
        sort_key=42,
    )
    generated_api_client.client.create_or_update_category(EVENT_ID, category.id, category)

    result = generated_api_client.client.list_categories(EVENT_ID)
    assert len(result) == 2

    generated_api_client.client.delete_category(EVENT_ID, category.id)

    result = generated_api_client.client.list_categories(EVENT_ID)
    assert len(result) == 1
    assert result[0].title == "Default"


def test_delete_category_with_replacement(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")
    # given: a new category and an entry, which belongs to this category
    category = kueaplan_api_client.Category(
        id=str(uuid.uuid4()),
        title="Test Category",
        icon="💡",
        color="ffaa00",
        sort_key=42,
    )
    generated_api_client.client.create_or_update_category(event_id, category.id, category)
    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        room=[],
        responsible_person="Max Mustermann",
        category=category.id,
        previousDates=[],
    )
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    # when: we delete the category and specify the default category as replacement
    # then: the deletion succeeds
    generated_api_client.client.delete_category(
        event_id,
        category.id,
        kueaplan_api_client.DeleteCategoryRequest(replace_category="019774dc-81c4-7862-a9ba-63de3d726010"),
    )

    # and then: the entry is assigned to the default category
    new_entry = generated_api_client.client.get_entry(event_id, entry.id)
    assert new_entry.category == uuid.UUID("019774dc-81c4-7862-a9ba-63de3d726010")


def test_delete_category_errors(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    generated_api_client.login(event_id, "orga")

    # Non-existing category
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_category(event_id, "11111111-2222-3333-4444-555555555555")
    assert "not exist" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 404

    # Last category of an event should not be deleted
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        # Default category from minimal.sql
        generated_api_client.client.delete_category(event_id, "019774dc-81c4-7862-a9ba-63de3d726010")
    assert "last category" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409

    # Create a new category and entry for testing
    category = kueaplan_api_client.Category(
        id=str(uuid.uuid4()),
        title="Test Category",
        icon="💡",
        color="ffaa00",
        sort_key=42,
    )
    generated_api_client.client.create_or_update_category(event_id, category.id, category)
    entry = kueaplan_api_client.Entry(
        id=str(uuid.uuid4()),
        title="Drachenfliegen leicht gemacht",
        begin=datetime.datetime(2025, 1, 6, 12, 0, tzinfo=datetime.UTC).isoformat(),
        end=datetime.datetime(2025, 1, 6, 13, 30, tzinfo=datetime.UTC).isoformat(),
        room=[],
        responsible_person="Max Mustermann",
        category=category.id,
        previousDates=[],
    )
    generated_api_client.client.create_or_update_entry(event_id, entry.id, entry)

    # Category referenced by entry and no replacement given
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_category(event_id, category.id)
    assert "referenced by entries" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409

    # Non-existing replacement
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_category(
            event_id,
            category.id,
            kueaplan_api_client.DeleteCategoryRequest(replace_category="11111111-2222-3333-4444-555555555555"),
        )
    assert "must reference an existing category" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409

    # Category replaced by itself
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_category(
            event_id, category.id, kueaplan_api_client.DeleteCategoryRequest(replace_category=category.id)
        )
    assert excinfo.value.data.http_code == 409

    # Unauthorized
    del generated_api_client.client.api_client.configuration.api_key["sessionTokenAuth"]
    generated_api_client.login(event_id, "user")
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.delete_category(event_id, category.id)
    assert "not authorized" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 403


def test_category_id_conflicts(generated_api_client: ApiClientWrapper, reset_database: None) -> None:
    import kueaplan_api_client

    event_id = 1
    other_event_id = 2

    generated_api_client.login(event_id, "orga")
    generated_api_client.login(other_event_id, "orga")

    category = kueaplan_api_client.Category(
        id=str(uuid.uuid4()),
        title="Test Category",
        icon="💡",
        color="ffaa00",
        sort_key=42,
    )
    generated_api_client.client.create_or_update_category(event_id, category.id, category)

    # Same category in other event
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_category(other_event_id, category.id, category)
    assert "already exists" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409

    # category has been deleted
    generated_api_client.client.delete_category(event_id, category.id)
    with pytest.raises(kueaplan_api_client.ApiException) as excinfo:
        generated_api_client.client.create_or_update_category(other_event_id, category.id, category)
    assert "already exists" in str(excinfo.value.data.message)
    assert excinfo.value.data.http_code == 409
