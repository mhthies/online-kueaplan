import datetime

from .actions import Announcement, AnnouncementType, Category, Entry, Room

CATEGORY_SPORT = Category(
    title="Sport",
    icon="⚽",
    color="#1c71d8",
    sort_key=30,
)

ROOM_SPORTPLAETZE = Room(title="Sportplätze")
ROOM_PELIKANHALLE = Room(title="Pelikanhalle")
ROOM_SEMINARRAUM = Room(title="Seminarraum Pelikanhalle")


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
ENTRY_LOREM_IPSUM = Entry(
    title="Lorem Ipsum dolor sit amet",
    day=datetime.date(2025, 1, 4),
    begin=datetime.time(14, 0),
    duration=datetime.timedelta(hours=1),
    comment="Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.",
    responsible_person="incognita",
    room_comment="Quis aute iure reprehenderit in voluptate velit esse.",
    description="""
Vulputate qui blandit praesent
==============================

Duis autem vel eum iriure dolor in hendrerit in vulputate velit esse molestie consequat, vel illum [dolore eu feugiat \
nulla facilisis](https://example.com) at vero eros et accumsan et iusto odio dignissim qui blandit praesent luptatum \
zzril delenit augue duis dolore te feugait nulla facilisi. Lorem ipsum dolor sit amet, consectetuer adipiscing elit, \
sed diam nonummy nibh euismod tincidunt ut laoreet dolore magna aliquam erat volutpat:

* At vero eos et accusam
* Justo duo dolores et ea rebum
* consetetur sadipscing elitr.


Ullamcorper lobortis
--------------------
Ut wisi enim ad minim veniam, quis nostrud exerci tation ullamcorper suscipit lobortis nisl ut aliquip ex ea commodo \
consequat. Duis autem vel eum iriure dolor in hendrerit in vulputate velit esse molestie consequat, vel illum dolore \
eu feugiat nulla facilisis at vero eros et accumsan et iusto odio dignissim qui blandit praesent luptatum zzril \
delenit augue duis dolore te feugait nulla facilisi.
""",
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
