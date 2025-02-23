#!/bin/env python3
import dataclasses
import datetime
import enum
import json
import re
import sys
import uuid
from pathlib import Path
from typing import Optional, Any, Iterable, TypeAlias, Mapping

import yaml
import pytz

JSON: TypeAlias = dict[str, "JSON"] | list["JSON"] | str | int | float | bool | None


def main() -> int:
    INPUT_PATH = Path(sys.argv[1])
    OUTPUT_PATH = Path(sys.argv[2])
    config, entries = read_data(INPUT_PATH)
    room_names = gather_rooms(entries)
    room_result, room_map = generate_json_rooms(room_names)
    result = {
        "entries": generate_json_entries(entries, room_map),
        "rooms": room_result,
        "categories": generate_json_categories(),
        "event": generate_json_event(config)
    }
    with OUTPUT_PATH.open("w") as f:
        json.dump(result, f, indent=2)
    return 0


def read_data(data_directory: Path) -> tuple[Any, list["KueaPlanEntry"]]:
    configfile = data_directory / "config.yaml"
    with configfile.open() as f:
        config = yaml.safe_load(f)
    entries = []
    index = 0
    for datafile in data_directory.glob("*.yaml"):
        match = re.match(r"(?P<year>\d+)-(?P<month>\d+)-(?P<day>\d+)", datafile.name)
        if not match:
            continue
        date = datetime.date(int(match["year"]), int(match["month"]), int(match["day"]))
        with datafile.open() as f:
            yaml_stream = yaml.safe_load_all(f)
            for doc in yaml_stream:
                if doc:
                    entries.append(KueaPlanEntry.from_yaml(doc, date, index))
                    index += 1
    return config, entries


# Copied from simple_kuaplan/kuea_data.py
EARLIEST_REASONABLE_KUEA = 5 * 60 + 30


class KueaType(enum.Enum):
    NORMAL = 0
    PLENUM = 1
    BLOCKER = 2
    ORGA = 3
    CANCELLED = 4

    @classmethod
    def from_yaml(cls, value: Optional[str]) -> "KueaType":
        if not value:
            return KueaType.NORMAL
        return cls[value.upper()]


@dataclasses.dataclass
class KueaPlanEntry:
    str_id: str
    title: str
    comment: Optional[str] = None
    people: list[str] = dataclasses.field(default_factory=list)
    place: str = ""
    place_comment: Optional[str] = None
    kuea_type: KueaType = KueaType.NORMAL
    id: Optional[uuid.UUID] = None
    begin: Optional[datetime.datetime] = None
    end: Optional[datetime.datetime] = None
    time_comment: Optional[str] = None
    description: str = ""

    @classmethod
    def from_yaml(cls, data: Any, date: datetime.date, index: int) -> "KueaPlanEntry":
        people = []
        if "people" in data:
            if isinstance(data["people"], str):
                people = [data["people"]]
            else:
                people = data["people"]
        begin_raw = data["begin"]
        begin = None
        if begin_raw is not None:
            if isinstance(begin_raw, int):
                begin = datetime.datetime(date.year, date.month, date.day, begin_raw // 60, begin_raw % 60, 0)
                if begin_raw <= EARLIEST_REASONABLE_KUEA:
                    begin += datetime.timedelta(days=1)
            elif isinstance(begin_raw, datetime.datetime):
                begin = begin_raw
            else:
                raise TypeError(f"'begin' has invalid type '{type(begin_raw).__name__}'")
        end_raw = data.get("end")
        end = None
        if end_raw is not None:
            if isinstance(end_raw, int):
                end = datetime.datetime(date.year, date.month, date.day, (end_raw // 60) % 24, end_raw % 60, 0)
                if end_raw <= EARLIEST_REASONABLE_KUEA:
                    end += datetime.timedelta(days=1)
            elif isinstance(end_raw, datetime.datetime):
                end = end_raw
            else:
                raise TypeError(f"'end' has invalid type '{type(end_raw)}'")
        # TODO uuid
        place = data.get("place")
        if place is None:
            place = ""
        return cls(
            str_id=f"{date:%Y%m%d}-{index}",
            title=data["title"],
            comment=data.get("comment"),
            people=people,
            begin=begin,
            end=end,
            time_comment=data.get("time_comment"),
            place=place,
            place_comment=data.get("place_comment"),
            kuea_type=KueaType.from_yaml(data.get("type")),
            description=data.get("description", ""),
        )


def get_rooms(entry: KueaPlanEntry) -> list[str]:
    return list(filter(lambda r: bool(r), re.split(r", | und | / ", entry.place.lstrip("Treffpunkt: "))))


def gather_rooms(entries: Iterable[KueaPlanEntry]) -> set[str]:
    rooms = set()
    for entry in entries:
        for room in get_rooms(entry):
            rooms.add(room)
    return rooms


def generate_json_rooms(room_names: Iterable[str]) -> tuple[list[dict[str, JSON]], dict[str, uuid]]:
    room_id_mapping = {room_name: uuid.uuid4() for room_name in room_names}
    json_output = [{"id": str(room_id), "title": room_name, "description": ""}
                   for room_name, room_id in room_id_mapping.items()]
    return json_output, room_id_mapping


CATEGORIES = {
    KueaType.NORMAL: {
        "id": "5def897e-5b43-40f8-bc7f-5be76d6ce15e",
        "title": "KüA",
        "icon": "⚽",
        "color": "0000ff",
    },
    KueaType.PLENUM: {
        "id": "483214e1-6e04-4db6-bb9e-af01e6d8b988",
        "title": "Plenum",
        "icon": "🧑‍🤝‍🧑",
        "color": "000000",
    },
    KueaType.ORGA: {
        "id": "615c732b-2182-4ccc-b5af-4004e85e0f9e",
        "title": "Orga",
        "icon": "📝",
        "color": "888888",
    },
}


def generate_json_categories() -> list[dict[str, JSON]]:
    return list(CATEGORIES.values())


def generate_json_entries(entries: Iterable[KueaPlanEntry], room_id_mapping: Mapping[str, uuid]) -> list[
    dict[str, JSON]]:
    result = []
    for entry in entries:
        EST = pytz.timezone('Europe/Berlin')
        begin = EST.localize(entry.begin)
        end = EST.localize(entry.end or entry.begin + datetime.timedelta(hours=1))
        result.append({
            "id": str(uuid.uuid4()),
            "title": entry.title,
            "comment": entry.comment or "",
            "description": entry.description,
            "begin": begin.isoformat(),
            "end": end.isoformat(),
            "timeComment": entry.time_comment or "",
            "room": [str(room_id_mapping.get(room)) for room in get_rooms(entry) if room in room_id_mapping],
            "roomComment": entry.place_comment or "",
            "responsiblePerson": ",".join(entry.people),
            "isRoomReservation": entry.kuea_type == KueaType.BLOCKER,
            "isExclusive": entry.kuea_type == KueaType.PLENUM,
            "isCancelled": entry.kuea_type == KueaType.CANCELLED,
            "category": CATEGORIES.get(entry.kuea_type, CATEGORIES[KueaType.NORMAL])["id"],
        })
    return result


def generate_json_event(config: Any) -> JSON:
    return {
        "id": -1,
        "title": config["event"]["title"],
        "begin_date": config["event"]["begin"].isoformat(),
        "end_date": config["event"]["end"].isoformat(),
    }

if __name__ == '__main__':
    sys.exit(main())
