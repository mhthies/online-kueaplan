#!/usr/bin/env python3
"""A simple helper script for modifying an exported event JSON file to shift all date and time values by a fixed number
of days.

This can be used to shift a test data file to the current date before importing it, such that features that rely on the
current date and time (e.g. the highlighting of the current day in the date dropdown menu) can be tested.
"""

import argparse
import datetime
import json
from pathlib import Path
from typing import Any, TypeAlias

JsonType: TypeAlias = None | int | str | bool | list["JsonType"] | dict[str, "JsonType"]


def main() -> None:
    args = parse_cli_args()
    with args.input_file.open() as f:
        data = json.load(f)

    timedelta = datetime.timedelta()
    if args.start_date is not None:
        current_start_date = datetime.date.fromisoformat(data["event"]["beginDate"])
        timedelta += args.start_date - current_start_date

    print(f"shifting by {timedelta}")
    shift_timestamps(data, timedelta)
    with args.output_file.open("w") as f:
        json.dump(data, f, indent=4)


def parse_cli_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_file", type=Path)
    parser.add_argument("output_file", type=Path)
    parser.add_argument(
        "--start-date",
        "-s",
        type=datetime.date.fromisoformat,
        help="Shift all the timestamps in the event, such that the first day of the event is the given date. "
        "Date must be given in RFC3339 format (e.g. 2026-05-17).",
    )
    return parser.parse_args()


def shift_timestamps(data: dict[str, Any], timedelta: datetime.timedelta) -> None:
    data["event"]["beginDate"] = _shift_date(data["event"]["beginDate"], timedelta)
    data["event"]["endDate"] = _shift_date(data["event"]["endDate"], timedelta)
    for entry in data["entries"]:
        entry["begin"] = _shift_datetime(entry["begin"], timedelta)
        entry["end"] = _shift_datetime(entry["end"], timedelta)
        for previous_date in entry["previousDates"]:
            previous_date["begin"] = _shift_datetime(previous_date["begin"], timedelta)
            previous_date["end"] = _shift_datetime(previous_date["end"], timedelta)
    for announcement in data.get("announcements", []):
        if announcement.get("beginDate") is not None:
            announcement["beginDate"] = _shift_date(announcement["beginDate"], timedelta)
        if announcement.get("endDate") is not None:
            announcement["endDate"] = _shift_date(announcement["endDate"], timedelta)


def _shift_date(value: str, timedelta: datetime.timedelta) -> str:
    date = datetime.date.fromisoformat(value)
    date += timedelta
    return date.isoformat()


def _shift_datetime(value: str, timedelta: datetime.timedelta) -> str:
    timestamp = datetime.datetime.fromisoformat(value)
    timestamp += timedelta
    return timestamp.isoformat()


if __name__ == "__main__":
    main()
