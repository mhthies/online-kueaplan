#!/bin/env/python3
import os
import time
import uuid

import requests

DEFAULT_SERVER_URL = "https://kueaplan.de"


def main():
    server_url = input(f"Server [{DEFAULT_SERVER_URL}]: ") or DEFAULT_SERVER_URL
    event_id = int(input("Event-ID: "))
    passphrase = input("Orga-Passphrase: ")
    base_url = f"{server_url}/api/v1"

    r = requests.post(f"{base_url}/events/{event_id}/auth", json={"passphrase": passphrase})
    r.raise_for_status()
    auth_token = r.json()["sessionToken"]

    for category in CATEGORIES:
        category_id = uuidv7()
        requests.put(
            f"{base_url}/events/{event_id}/categories/{category_id}",
            headers={
                "X-SESSION-TOKEN": auth_token,
            },
            json={"id": str(category_id), **category},
        ).raise_for_status()


CATEGORIES = [
    {"title": "KüA, allgemein", "icon": "", "color": "99aabb", "sort_key": -10},
    {
        "title": "Literatur & Sprache",
        "icon": "\ud83d\udcd6",
        "color": "20c1b0",
        "sort_key": 10,
    },
    {"title": "Kreatives", "icon": "\ud83c\udfa8", "color": "3eff68", "sort_key": 15},
    {"title": "Musizieren", "icon": "\ud83c\udfb5", "color": "33d17a", "sort_key": 20},
    {"title": "Tanzen", "icon": "\ud83d\udc83", "color": "1c71d8", "sort_key": 25},
    {"title": "Sport", "icon": "\u26bd", "color": "1c71d8", "sort_key": 30},
    {"title": "Spiele", "icon": "\u265f\ufe0f", "color": "f5c211", "sort_key": 35},
    {
        "title": "Gespr\u00e4chsrunde",
        "icon": "\ud83d\udde8\ufe0f",
        "color": "ff7800",
        "sort_key": 40,
    },
    {
        "title": "Auff\u00fchrung",
        "icon": "\ud83c\udfad",
        "color": "e01b7e",
        "sort_key": 50,
    },
    {
        "title": "Vortrag",
        "icon": "\ud83e\uddd1\u200d\ud83c\udf93",
        "color": "ba2aca",
        "sort_key": 55,
    },
    {
        "title": "CdE-Vereinsarbeit",
        "icon": "\ud83d\udca1",
        "color": "4a2269",
        "sort_key": 60,
    },
    {
        "title": "Plenum",
        "icon": "\ud83d\udce2",
        "color": "12143b",
        "isOfficial": True,
        "sort_key": 100,
    },
    {
        "title": "Orga",
        "icon": "\ud83d\udcce",
        "color": "233d37",
        "isOfficial": True,
        "sort_key": 110,
    },
]


def uuidv7() -> uuid.UUID:
    """Generate a UUID v7.

    Polyfill, while waiting for official UUID v7 support in Python 3.14 (https://github.com/python/cpython/pull/121119).
    Source: https://github.com/python/cpython/pull/121119
    """
    # random bytes
    value = bytearray(os.urandom(16))

    # current timestamp in ms
    timestamp = int(time.time() * 1000)

    # timestamp
    value[0] = (timestamp >> 40) & 0xFF
    value[1] = (timestamp >> 32) & 0xFF
    value[2] = (timestamp >> 24) & 0xFF
    value[3] = (timestamp >> 16) & 0xFF
    value[4] = (timestamp >> 8) & 0xFF
    value[5] = timestamp & 0xFF

    # version and variant
    value[6] = (value[6] & 0x0F) | 0x70
    value[8] = (value[8] & 0x3F) | 0x80

    result = uuid.UUID(bytes=bytes(value))
    assert result.variant == uuid.RFC_4122
    assert result.version == 7
    return result


if __name__ == "__main__":
    main()
