import datetime

from .actions import Announcement, AnnouncementType, Category, Entry, Room

CATEGORY_SPORT = Category(
    title="Sport",
    icon="⚽",
    color="#1c71d8",
    sort_key=30,
)

ROOM_SPORTPLAETZE = Room(title="Sportplätze")

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
    begin=datetime.time(5, 15),
    duration=datetime.timedelta(hours=1, minutes=30),
    comment="Minderjährige können mitgehen",
    responsible_person="Sören",
    room_comment="Treffpunkt: Orgabüro",
)
ENTRY_AKROBATIK = Entry(
    title="Akrobatik für Fortgeschrittene",
    comment="Voraussetzung: Erfahrung >1 Akrobatik KüA",
    day=datetime.date(2025, 1, 3),
    begin=datetime.time(21, 0),
    duration=datetime.timedelta(hours=1),
    responsible_person="Lilo Thiemann",
)
ENTRY_TANZABEND = Entry(
    title="Tanzabend",
    comment="Standard/Latein",
    day=datetime.date(2025, 1, 3),
    begin=datetime.time(21, 0),
    duration=datetime.timedelta(hours=4),
    responsible_person="Lore",
)
ENTRY_WEST_COAST_SWING = Entry(
    title="West Coast Swing – Freies Tanzen",
    day=datetime.date(2025, 1, 3),
    begin=datetime.time(0, 15),
    duration=datetime.timedelta(hours=1, minutes=45),
    responsible_person="Mina Janßen und Ivan Kugler",
)

ANNOUNCEMENT_SPORTPLATZ_NASS = Announcement(
    text="Achtung: Auf dem Sportplatz ist es nass und rutschig. Bitte aufpassen.",
    type=AnnouncementType.WARNING,
    show_with_days=True,
    begin_date=datetime.date(2025, 1, 4),
    show_with_rooms=True,
    rooms=["Sportplätze"],
)
