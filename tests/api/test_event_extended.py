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


def test_update_extended_event(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    generated_api_client.login(EVENT_ID, "admin")
    event_info = generated_api_client.client.get_extended_event_info(EVENT_ID)

    event_info.begin_date = datetime.date(2025, 1, 2)
    event_info.timezone = "Europe/London"
    assert event_info.default_time_schedule.sections[3].name == "Abends"
    event_info.default_time_schedule.sections[3].end_time = "01:00:00"
    event_info.default_time_schedule.sections.append(generated_api_client.module.EventDayTimeScheduleSectionsInner(
        name="Nachts", end_time=None))
    generated_api_client.client.update_extended_event(EVENT_ID, event_info)

    new_event_info = generated_api_client.client.get_extended_event_info(EVENT_ID)
    assert new_event_info == event_info

def test_update_extended_event_errors(generated_api_client: ApiClientWrapper) -> None:
    EVENT_ID = 1
    # Not authorized as orga
    generated_api_client.login(EVENT_ID, "orga")
    event_info = generated_api_client.client.get_extended_event_info(EVENT_ID)
    with pytest.raises(Exception) as excinfo:
        generated_api_client.client.update_extended_event(EVENT_ID, event_info)
    assert excinfo.value.status == 403
    assert "Authentication as Admin is required" in str(excinfo.value.data.message)

    # Invalid timezone
    generated_api_client.login(EVENT_ID, "admin")
    event_with_invalid_timezone = event_info.model_copy(update={"timezone": "Europe/Aachen"})
    with pytest.raises(Exception) as excinfo:
        generated_api_client.client.update_extended_event(EVENT_ID, event_with_invalid_timezone)
    assert excinfo.value.status == 422
    assert "timezone" in str(excinfo.value.data.message)

    # Begin after end
    assert event_info.end_date == datetime.date(2025,1, 6)
    event_with_invalid_timezone = event_info.model_copy(update={"begin_date": datetime.date(2025, 1, 8)})
    with pytest.raises(Exception) as excinfo:
        generated_api_client.client.update_extended_event(EVENT_ID, event_with_invalid_timezone)
    assert excinfo.value.status == 422
    assert "begin_date" in str(excinfo.value.data.message)

    # Invalid time schedule sections
    event_with_invalid_schedule_section = event_info.model_copy()
    event_with_invalid_schedule_section.default_time_schedule.sections.insert(
        2, generated_api_client.module.EventDayTimeScheduleSectionsInner(name="Vormittags", end_time="10:00:00"))
    with pytest.raises(Exception) as excinfo:
        generated_api_client.client.update_extended_event(EVENT_ID, event_with_invalid_schedule_section)
    assert excinfo.value.status == 422
    assert "Schedule sections" in str(excinfo.value.data.message)
