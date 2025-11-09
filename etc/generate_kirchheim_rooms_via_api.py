#!/bin/env/python3
import os
import time
import uuid

import requests


def main():
    event_id = int(input("Event-ID: "))
    passphrase = input("Orga-Passphrase: ")

    rooms = GENERAL_ROOMS + [f"{yard} {room}" for yard in YARDS for room in YARD_ROOMS]

    r = requests.post(f"{BASE_URL}/events/{event_id}/auth", json={"passphrase": passphrase})
    r.raise_for_status()
    auth_token = r.json()["sessionToken"]

    for room in rooms:
        room_id = uuidv7()
        requests.put(
            f"{BASE_URL}/events/{event_id}/rooms/{room_id}",
            headers={
                "X-SESSION-TOKEN": auth_token,
            },
            json={
                "id": str(room_id),
                "title": room,
                "description": "",
            },
        ).raise_for_status()


BASE_URL = "https://kueaplan.de/api/v1"

GENERAL_ROOMS = [
    "Pelikanhalle",
    "Pelikanhalle Empore",
    "Seminarraum Pelikanhalle",
    "Seminarraum Krankenstation",
    "Dorfkrug",
    "Werkraum",
    "Badesee",
    "Sportplätze",
    "Amphitheater/Flaggen",
    "Spielplatz",
    "vor der Pelikanhalle",
    "Boulderwand",
]

YARDS = [
    "Buchwaldhof",
    "Steinsgebisshof",
    "Lischerthof",
    "Löscherhof",
    "CdErnhof",
]

YARD_ROOMS = [
    "Speisesaal",
    "Hof",
    "Feuerstelle",
    "Haus 1",
    "Haus 2",
    "Haus 3",
    "Haus 4",
    "ObÄ",
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
