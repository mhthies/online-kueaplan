import dataclasses
import re

from playwright.sync_api import Page, expect

from tests.ui import actions, data
from tests.ui.helpers import get_table_row_by_column_value, is_text_bold

# We don't need to test creating categories here, as this is covered by many tests, using the `actions.add_category()`
# function.


def test_change_category(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_category(page, data.CATEGORY_SPORT)
    actions.add_entry(page, dataclasses.replace(data.ENTRY_BEACH_VOLLEYBALL, rooms=[], room_comment=""))

    page.get_by_role("link", name="Konfiguration").click()
    page.get_by_role("link", name="KüA-Kategorien").click()
    sport_row = get_table_row_by_column_value(page, "Name", "Sport")
    sport_row.get_by_role("link", name="Bearbeiten").click()

    page.get_by_role("checkbox", name="ist offiziell").check()
    page.get_by_role("textbox", name="Name der Kategorie").fill("Außensport")
    page.get_by_role("textbox", name="Icon").fill("🎾")
    page.get_by_role("button", name="Speichern").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()

    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Sa 04.01.").click()
    entry_row = get_table_row_by_column_value(page, "Was?", "Beach-Volleyball")

    expect(entry_row).to_contain_text("🎾")
    # We changed to an "official" category, so the entry should be displayed in bold font
    assert is_text_bold(entry_row.get_by_text("Beach-Volleyball"))

    page.get_by_role("link", name="Kategorien").click()
    expect(page.get_by_text("Außensport")).to_be_visible()
    expect(page.get_by_text("Außensport")).to_contain_text("🎾")
    expect(page.get_by_text("Default")).to_be_visible()
    expect(page.get_by_text("Default")).not_to_contain_text("🎾")


def test_category_sorting(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_category(page, actions.Category("Plenum", "📢", "#12143b", -10, True))
    actions.add_category(page, actions.Category("Musizieren", "🎵", "#33d17a", 10, False))

    expect(page.locator("xpath=//td[2]")).to_contain_text(["Plenum", "Default", "Musizieren"])
    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Kategorien").click()
    expect(page.get_by_role("link")).to_contain_text(["Plenum", "Default", "Musizieren"])

    page.get_by_role("link", name="Kategorien verwalten").click()
    plenum_row = get_table_row_by_column_value(page, "Name", "Plenum")
    plenum_row.get_by_role("link", name="Bearbeiten").click()
    page.get_by_role("spinbutton", name="Sortier-Schlüssel").fill("10")
    page.get_by_role("button", name="Speichern").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()

    expect(page.locator("xpath=//td[2]")).to_contain_text(["Default", "Musizieren", "Plenum"])
    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Kategorien").click()
    expect(page.get_by_role("link")).to_contain_text(["Default", "Musizieren", "Plenum"])


def test_delete_category(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_category(page, actions.Category("Plenum", "📢", "#12143b", 20, True))
    actions.add_category(page, data.CATEGORY_SPORT)
    actions.add_entry(page, dataclasses.replace(data.ENTRY_BEGRUESSUNGSPLENUM, category="Plenum", rooms=[]))
    actions.add_entry(page, dataclasses.replace(data.ENTRY_BEACH_VOLLEYBALL, rooms=[]))

    page.get_by_role("link", name="Konfiguration").click()
    page.get_by_role("navigation", name="Konfigurationsbereich-Navigation").get_by_role(
        "link", name="KüA-Kategorien"
    ).click()
    plenum_row = get_table_row_by_column_value(page, "Name", "Plenum")
    plenum_row.get_by_role("link", name="Löschen").click()

    expect(page.get_by_role("region", name="Zu löschende Kategorie")).to_contain_text("Plenum")
    expect(page.get_by_role("region", name="Zu löschende Kategorie")).to_contain_text("📢")
    expect(page.get_by_text("Betroffene Einträge")).to_be_visible()
    expect(page.get_by_role("region", name="Betroffene Einträge")).to_contain_text("Anzahl: 1")
    expect(page.get_by_role("region", name="Betroffene Einträge")).to_contain_text(
        re.compile(r".*Begrüßungsplenum.*Orgas.*01\.01\.\s*20:00.*", re.DOTALL)
    )
    expect(page.get_by_role("region", name="Betroffene Einträge")).not_to_contain_text("Beach-Volleyball")
    page.get_by_role("combobox", name="Einträge in folgende Kategorie verschieben").select_option(label="Default")
    page.get_by_role("button", name="Kategorie löschen").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()

    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Sa 04.01.").click()
    entry_row = get_table_row_by_column_value(page, "Was?", "Beach-Volleyball")
    expect(entry_row).to_contain_text("⚽")

    page.get_by_role("link", name="Kategorien").click()
    expect(page.get_by_role("link", name="Sport")).to_be_visible()
    expect(page.get_by_role("link", name="Plenum")).not_to_be_visible()
    page.get_by_role("link", name="Default").click()
    entry_row = get_table_row_by_column_value(page, "Was?", "Begrüßungsplenum")
    expect(entry_row).to_be_visible()
