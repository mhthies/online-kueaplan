import datetime

from tests.conftest import ApiClientWrapper


def test_get_event(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    result = generated_api_client.client.get_event_info(EVENT_ID)
    assert result.id == 1
    assert result.title == "TestEvent"
    assert result.begin_date == datetime.date(2025,1,1)
    assert result.end_date == datetime.date(2025,1, 6)


def test_list_events(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    result = generated_api_client.client.list_events(EVENT_ID)
    assert len(result) == 1
    assert result[0].title == "TestEvent"

    result = generated_api_client.client.list_events(EVENT_ID, after=datetime.date(2025, 5, 1))
    assert len(result) == 0
