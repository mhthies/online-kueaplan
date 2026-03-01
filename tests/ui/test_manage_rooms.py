import dataclasses
import re

from playwright.sync_api import Page, expect

from tests.ui import actions, data
from tests.ui.helpers import get_table_cell_by_header, get_table_row_by_column_value

# We don't need to test creating rooms here, as this is covered by many tests, using the `actions.add_room()` function.


def test_change_room(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_room(page, data.ROOM_PELIKANHALLE)
    actions.add_entry(page, data.ENTRY_BEGRUESSUNGSPLENUM)

    page.get_by_role("link", name="Konfiguration").click()
    page.get_by_role("navigation", name="Konfigurationsbereich-Navigation").get_by_role("link", name="Orte").click()
    plenum_row = get_table_row_by_column_value(page, "Name", "Pelikanhalle").filter(has_not_text="Seminarraum")
    plenum_row.get_by_role("link", name="Bearbeiten").click()
    page.get_by_role("textbox", name="Name des Orts").fill("Pelikanhalle (unten)")
    page.get_by_role("textbox", name="Beschreibung").fill("""
# Informationen

Die Pelikanhalle ist eine Mutifunktions-Sporthalle.

# Wegbeschreibung

Vom Parkplatz aus durch die Schranke gehen und dem rechten Weg folgen.
Die Pelikanhalle liegt dann als drittes Gebäude auf der rechten Seite.
Nicht den ersten Eingang (am etwas zurückliegenden Gebäudeteil) nehmen, sondern die zweite Tür.
Dann rechts die Treppe runter gehen.
    """)
    page.get_by_role("button", name="Speichern").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()

    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Mi 01.01.").click()
    room_cell = get_table_cell_by_header(page.get_by_role("table"), "Wo?")
    expect(room_cell).to_contain_text("Pelikanhalle (unten)")

    page.get_by_role("link", name="Orte").click()
    page.get_by_role("link", name="Pelikanhalle (unten)").click()
    expect(page.get_by_role("heading")).to_contain_text(["Pelikanhalle (unten)", "Informationen", "Wegbeschreibung"])
    expect(page.get_by_text("Begrüßungsplenum")).to_be_visible()


def test_delete_room(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_room(page, data.ROOM_SEMINARRAUM)
    actions.add_room(page, data.ROOM_SPORTPLAETZE)
    actions.add_room(page, data.ROOM_PELIKANHALLE)
    actions.add_entry(
        page, dataclasses.replace(data.ENTRY_BEGRUESSUNGSPLENUM, room_comment="Bei schönem Wetter draußen")
    )
    actions.add_entry(page, dataclasses.replace(data.ENTRY_PLENUMSVORBEREITUNG))
    actions.add_entry(page, dataclasses.replace(data.ENTRY_BEACH_VOLLEYBALL, category=None))

    page.get_by_role("link", name="Konfiguration").click()
    page.get_by_role("navigation", name="Konfigurationsbereich-Navigation").get_by_role("link", name="Orte").click()
    room_row = get_table_row_by_column_value(page, "Name", "Pelikanhalle").filter(has_not_text="Seminarraum")
    room_row.get_by_role("link", name="Löschen").click()
    expect(page.get_by_role("region", name="Zu löschender Ort")).to_contain_text("Pelikanhalle")
    expect(page.get_by_text("Betroffene Einträge")).to_be_visible()
    expect(page.get_by_role("region", name="Betroffene Einträge")).to_contain_text("Anzahl: 2")
    expect(page.get_by_role("region", name="Betroffene Einträge").get_by_role("listitem")).to_contain_text(
        [
            re.compile(r".*Plenums-Vorbereitung.*Orgas.*01\.01\.\s*19:30.*", re.DOTALL),
            re.compile(r".*Begrüßungsplenum.*Orgas.*01\.01\.\s*20:00.*", re.DOTALL),
        ]
    )
    expect(page.get_by_role("region", name="Betroffene Einträge")).not_to_contain_text("Beach-Volleyball")
    page.get_by_role("button", name="Ort löschen").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()

    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Mi 01.01.").click()
    entry_row = get_table_row_by_column_value(page, "Was?", "Plenums-Vorbereitung")
    room_cell = get_table_cell_by_header(entry_row, "Wo?")
    expect(room_cell).to_have_text("Seminarraum Pelikanhalle")

    entry_row = get_table_row_by_column_value(page, "Was?", "Begrüßungsplenum")
    room_cell = get_table_cell_by_header(entry_row, "Wo?")
    expect(room_cell).not_to_contain_text("Pelikanhalle")
    expect(room_cell).to_contain_text("Bei schönem Wetter draußen")

    page.get_by_role("link", name="Orte").click()
    expect(page.get_by_role("link", name="Pelikanhalle", exact=True)).not_to_be_visible()
    page.get_by_role("link", name="Sportplätze").click()
    entry_row = get_table_row_by_column_value(page, "Was?", "Beach-Volleyball")
    expect(entry_row).to_be_visible()


def test_delete_room_with_replacement(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_room(page, actions.Room("Pelikanhalle (unten)"))
    actions.add_room(page, data.ROOM_SEMINARRAUM)
    actions.add_room(page, data.ROOM_SPORTPLAETZE)
    actions.add_room(page, data.ROOM_PELIKANHALLE)
    actions.add_entry(
        page,
        dataclasses.replace(
            data.ENTRY_BEGRUESSUNGSPLENUM, rooms=["Pelikanhalle (unten)"], room_comment="Bei schönem Wetter draußen"
        ),
    )
    actions.add_entry(
        page,
        dataclasses.replace(data.ENTRY_PLENUMSVORBEREITUNG, rooms=["Pelikanhalle (unten)", "Seminarraum Pelikanhalle"]),
    )
    actions.add_entry(page, dataclasses.replace(data.ENTRY_BEACH_VOLLEYBALL, category=None))

    page.get_by_role("link", name="Konfiguration").click()
    page.get_by_role("navigation", name="Konfigurationsbereich-Navigation").get_by_role("link", name="Orte").click()
    room_row = get_table_row_by_column_value(page, "Name", "Pelikanhalle (unten)")
    room_row.get_by_role("link", name="Löschen").click()
    expect(page.get_by_role("region", name="Zu löschender Ort")).to_contain_text("Pelikanhalle (unten)")

    page.get_by_role("combobox", name="Einträge in folgende Orte verschieben").fill("Pelikanhalle")
    page.get_by_role("option", name="Pelikanhalle", exact=True).click()
    page.get_by_role("textbox", name="Ergänzender Kommentar zum Ort bei den betroffenen Einträgen").fill(
        "unten in der Halle"
    )
    page.get_by_role("button", name="Ort löschen").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()

    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Mi 01.01.").click()
    entry_row = get_table_row_by_column_value(page, "Was?", "Plenums-Vorbereitung")
    room_cell = get_table_cell_by_header(entry_row, "Wo?")
    expect(room_cell).to_contain_text("Pelikanhalle, Seminarraum Pelikanhalle")
    expect(room_cell).to_contain_text("unten in der Halle")

    entry_row = get_table_row_by_column_value(page, "Was?", "Begrüßungsplenum")
    room_cell = get_table_cell_by_header(entry_row, "Wo?")
    expect(room_cell).to_contain_text("Pelikanhalle")
    expect(room_cell).to_contain_text("Bei schönem Wetter draußen")
    expect(room_cell).to_contain_text("unten in der Halle")

    page.get_by_role("link", name="Orte").click()
    expect(page.get_by_role("link", name="Pelikanhalle (unten)")).not_to_be_visible()
    page.get_by_role("link", name="Sportplätze").click()
    entry_row = get_table_row_by_column_value(page, "Was?", "Beach-Volleyball")
    expect(entry_row).to_be_visible()
