import dataclasses
import datetime
import enum
import re
from typing import Optional

from playwright.sync_api import Page, expect


def login(page: Page, event_id: int, passphrase: str) -> None:
    page.goto(f"http://localhost:9099/ui/{event_id}/login")
    expect(page).to_have_title(re.compile("Login"))
    page.get_by_role("textbox", name="Passphrase").fill(passphrase)
    page.get_by_role("button", name="Zum KüA-Plan").click()
    success_alert = page.get_by_role("alert").filter(has_text="Login erfolgreich")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()


@dataclasses.dataclass
class Entry:
    title: str
    day: datetime.date
    begin: datetime.time
    duration: datetime.timedelta
    comment: str = ""
    is_cancelled: bool = False
    responsible_person: str = ""
    category: Optional[str] = None
    time_comment: str = ""
    rooms: list[str] = dataclasses.field(default_factory=lambda: [])
    room_comment: str = ""
    is_room_reservation: bool = False
    is_exclusive: bool = False
    description: str = ""


def add_entry(page: Page, entry: Entry) -> None:
    page.get_by_role("link", name="Eintrag hinzufügen").click()
    page.get_by_role("textbox", name="Titel").fill(entry.title)
    page.get_by_role("textbox", name="Kommentar / Kurze Beschreibung").fill(entry.comment)
    if entry.is_cancelled:
        page.get_by_role("checkbox", name="fällt aus").check()
    page.get_by_role("textbox", name="von wem?").fill(entry.responsible_person)
    if entry.category is not None:
        page.get_by_role("combobox", name="Kategorie").select_option(label=entry.category)
    if entry.is_room_reservation:
        page.get_by_role("checkbox", name="ist ein Raum-Blocker").check()
    if entry.is_exclusive:
        page.get_by_role("checkbox", name="ist exklusiver Zeitslot").check()
    page.get_by_role("combobox", name="Tag").select_option(value=entry.day.strftime("%Y-%m-%d"))
    page.get_by_role("textbox", name="Beginn").fill(entry.begin.strftime("%H:%M"))
    page.get_by_role("textbox", name="Dauer").fill(f"{(entry.duration.total_seconds() / 3600):f}")
    page.get_by_role("textbox", name="Kommentar zur Zeit").fill(entry.time_comment)
    for room in entry.rooms:
        page.get_by_role("combobox", name="Orte").fill(room)
        page.get_by_role("option", name=room).click()
    page.get_by_role("textbox", name="Kommentar zum Ort").fill(entry.room_comment)
    page.get_by_role("textbox", name="Ausführliche Beschreibung").fill(entry.description)
    page.get_by_role("button", name="Erstellen").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()


@dataclasses.dataclass
class Room:
    title: str
    description: str = ""


def add_room(page: Page, room: Room) -> None:
    page.get_by_role("link", name="Konfiguration").click()
    page.get_by_role("navigation", name="Konfigurationsbereich-Navigation").get_by_role("link", name="Orte").click()
    page.get_by_role("link", name="Ort hinzufügen").click()
    page.get_by_role("textbox", name="Name des Orts").fill(room.title)
    page.get_by_role("textbox", name="Beschreibung").fill(room.description)
    page.get_by_role("button", name="Erstellen").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()


@dataclasses.dataclass
class Category:
    title: str
    icon: str = ""
    color: Optional[str] = None
    sort_key: Optional[int] = None
    is_official: bool = False


def add_category(page: Page, category: Category) -> None:
    page.get_by_role("link", name="Konfiguration").click()
    page.get_by_role("navigation", name="Konfigurationsbereich-Navigation").get_by_role(
        "link", name="KüA-Kategorien"
    ).click()
    page.get_by_role("link", name="Kategorie hinzufügen").click()
    page.get_by_role("textbox", name="Name der Kategorie").fill(category.title)
    page.get_by_role("textbox", name="Icon").fill(category.icon)
    if category.color is not None:
        page.get_by_role("textbox", name="Farbe").fill(category.color)
    if category.sort_key is not None:
        page.get_by_role("spinbutton", name="Sortier-Schlüssel").fill(str(category.sort_key))
    if category.is_official:
        page.get_by_role("checkbox", name="ist offiziell").check()
    page.get_by_role("button", name="Erstellen").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()


class AnnouncementType(enum.Enum):
    INFO = "Information"
    WARNING = "Warnung"


@dataclasses.dataclass
class Announcement:
    text: str
    type: AnnouncementType = AnnouncementType.INFO
    sort_key: int = 0
    show_with_days: bool = False
    begin_date: Optional[datetime.date] = None
    end_date: Optional[datetime.date] = None
    show_with_rooms: bool = False
    rooms: list[str] = dataclasses.field(default_factory=lambda: [])
    show_with_categories: bool = False
    categories: list[str] = dataclasses.field(default_factory=lambda: [])


def add_announcement(page: Page, announcement: Announcement) -> None:
    page.get_by_role("link", name="Konfiguration").click()
    page.get_by_role("navigation", name="Konfigurationsbereich-Navigation").get_by_role(
        "link", name="Bekanntmachungen"
    ).click()
    page.get_by_role("link", name="Bekanntmachung hinzufügen").click()
    page.get_by_role("combobox", name="Typ").select_option(label=announcement.type.value)
    page.get_by_role("spinbutton", name="Sortier-Schlüssel").fill(str(announcement.sort_key))
    page.get_by_role("textbox", name="Text").fill(announcement.text)
    if announcement.show_with_days:
        page.get_by_role("checkbox", name="Anzeigen im KüA-Plan nach Datum").check()
    if announcement.begin_date is not None:
        page.get_by_role("combobox", name="ab Datum").select_option(label=announcement.begin_date.strftime("%d.%m."))
    if announcement.end_date is not None:
        page.get_by_role("combobox", name="bis Datum").select_option(label=announcement.end_date.strftime("%d.%m."))
    if announcement.show_with_categories:
        page.get_by_role("checkbox", name="Anzeigen im KüA-Plan nach Kategorie").check()
    for room in announcement.categories:
        page.get_by_role("combobox", name="Kategorien").fill(room)
        page.get_by_role("option", name=room).click()
    if announcement.show_with_rooms:
        page.get_by_role("checkbox", name="Anzeigen im KüA-Plan nach Raum").check()
    for room in announcement.rooms:
        page.get_by_role("combobox", name="Räume").fill(room)
        page.get_by_role("option", name=room).click()

    page.get_by_role("button", name="Erstellen").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()
