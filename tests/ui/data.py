import datetime

from .actions import Entry, Category, Room

CATEGORY_SPORT = Category(
    title="Sport",
    icon="⚽",
    color="#1c71d8",
    sort_key=30,
)

ROOM_SPORTPLAETZE = Room(
    title="Sportplätze"
)

ENTRY_BEACH_VOLLEYBALL = Entry(
    title="Beach-Volleyball",
    day=datetime.date(2025, 1, 4),
    begin=datetime.time(13, 30),
    duration=datetime.timedelta(hours=1, minutes=30),
    comment="bitte Bälle und Musik mitbringen",
    category="Sport",
    responsible_person="Fabienne Wagener",
    rooms=["Sportplätze"],
    room_comment="Beach-Volleyball-Feld",
)
ENTRY_SONNENAUFGANG_WANDERUNG = Entry(
    title="Sonnenaufgang-Wanderung",
    day=datetime.date(2025, 1, 5),
    begin=datetime.time(5,15),
    duration=datetime.timedelta(hours=1, minutes=30),
    comment="Minderjährige können mitgehen",
    responsible_person="Sören",
    room_comment="Treffpunkt: Orgabüro",
)
