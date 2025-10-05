import datetime

import pytest

from tests.conftest import ApiClientWrapper


def test_get_extended_event(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "user")
    result = generated_api_client.client.get_extended_event_info(EVENT_ID)
    assert result.id == 1
    assert result.title == "TestEvent"
    assert result.begin_date == datetime.date(2025,1,1)
    assert result.end_date == datetime.date(2025,1, 6)
    assert result.effective_begin_of_day == "05:30:00"
    assert result.timezone == "Europe/Berlin"
    assert len(result.default_time_schedule.sections) == 4
    assert result.default_time_schedule.sections[1].name == "Morgens"
    assert result.default_time_schedule.sections[1].end_time == "12:00:00"
    assert result.default_time_schedule.sections[3].end_time is None

def test_get_extended_event_errors(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    # Unauthenticated
    with pytest.raises(Exception) as excinfo:
        generated_api_client.client.get_extended_event_info(EVENT_ID)
    assert "requires authentication" in str(excinfo.value.data.message)
