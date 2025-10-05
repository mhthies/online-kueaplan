import datetime

from tests.conftest import ApiClientWrapper


def test_get_extended_event(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    result = generated_api_client.client.get_event_info(EVENT_ID)
    assert result.id == 1
    assert result.title == "TestEvent"
    assert result.begin_date == datetime.date(2025,1,1)
    assert result.end_date == datetime.date(2025,1, 6)
