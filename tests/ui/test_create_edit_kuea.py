import re
from playwright.sync_api import Page, expect

from . import actions


def test_create_entry(page: Page, reset_database: None):
    actions.login(page, 1, "orga")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    page.get_by_role("link", name="Eintrag hinzufügen").click()

    expect(page).to_have_title(re.compile(r"Neuer Eintrag"))
    expect(page.get_by_role("heading", name="Neuer Eintrag")).to_be_visible()

    page.get_by_role("textbox", name="Titel").fill("Drachenfliegen leicht gemacht")
    page.get_by_role("textbox", name="Kommentar / Kurze Beschreibung").fill("wir lassen Drachen steigen")
    page.get_by_role("textbox", name="von wem?").fill("Max Mustermann")
    page.get_by_role("textbox", name="Beginn").fill("13:00")
    page.get_by_role("textbox", name="Dauer").fill("1,5")
    page.get_by_role("textbox", name="Kommentar zur Zeit").fill("Direkt nach dem Mittagessen")
    page.get_by_role("textbox", name="Ausführliche Beschreibung").fill(
        """Wir bauen Drachen und lassen sie steigen.
        
        Für das Material müssen von jedem Teilnehmer an der KüA **5€** bezahlt werden.
        """)
    page.get_by_role("button", name="Speichern").click()
    expect(page).to_have_title(re.compile(r"06\.01\."))

    # TODO room

    main_table = page.get_by_role("table")
    expect(main_table.get_by_role("row")).to_have_count(2)
    row = main_table.get_by_role("row").nth(1)
    expect(row.get_by_role("cell").nth(0)).to_contain_text("Drachenfliegen leicht gemacht")
    expect(row.get_by_role("cell").nth(0)).to_contain_text("wir lassen Drachen steigen")
    expect(row.get_by_role("cell").nth(1)).to_contain_text("13:00 – 14:30")
    expect(row.get_by_role("cell").nth(3)).to_contain_text("Max Mustermann")


def test_create_entry_prefilled_date(page: Page, reset_database: None):
    actions.login(page, 1, "orga")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    page.get_by_role("button", name="Mo 06.01.").click()  # open date dropdown
    page.get_by_role("link", name="Do 02.01.").click()
    expect(page).to_have_title(re.compile(r"02\.01\."))

    page.get_by_role("link", name="Eintrag hinzufügen").click()
    expect(page.get_by_role("combobox", name="Tag")).to_have_value("2025-01-02")


def test_create_entry_validation_error_duration(page: Page, reset_database: None):
    actions.login(page, 1, "orga")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    page.get_by_role("link", name="Eintrag hinzufügen").click()

    expect(page).to_have_title(re.compile(r"Neuer Eintrag"))
    expect(page.get_by_role("heading", name="Neuer Eintrag")).to_be_visible()

    page.get_by_role("textbox", name="Titel").fill("Drachenfliegen leicht gemacht")
    page.get_by_role("textbox", name="von wem?").fill("Max Mustermann")
    page.get_by_role("textbox", name="Beginn").fill("13:00")
    page.get_by_role("textbox", name="Dauer").fill("1:")
    page.get_by_role("button", name="Speichern").click()

    error_alert = page.get_by_role("alert").filter(has_text="Eingegebene Daten sind ungültig")
    expect(error_alert).to_be_visible()
    expect(page).to_have_title(re.compile(r"Neuer Eintrag"))
    duration_input = page.get_by_role("textbox", name="Dauer")
    # parent of input field has 'is-invalid' class for red marker
    expect(duration_input.locator("..")).to_have_class(re.compile(r"(^|\s)is-invalid(\s|$)"))
    # error text within form row (parent of input group)
    expect(duration_input.locator("../..")).to_have_text(re.compile(r"Keine gültige Dauer"))


def test_create_entry_date_info_indicator(page: Page, reset_database: None):
    actions.login(page, 1, "orga")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    page.get_by_role("link", name="Eintrag hinzufügen").click()

    expect(page).to_have_title(re.compile(r"Neuer Eintrag"))
    date_input = page.get_by_role("combobox", name="Tag")
    begin_input = page.get_by_role("textbox", name="Beginn")

    date_input.select_option("03.01. (Fr)")
    begin_input.fill("02:00")
    calendar_date_indicator = begin_input.locator("..").locator("#calendarDateInfo")
    expect(calendar_date_indicator).to_be_visible()
    expect(calendar_date_indicator).to_have_text("04.01.")
    expect(calendar_date_indicator).to_have_css("color", "rgb(13, 202, 240)")

    date_input.select_option("04.01. (Sa)")
    expect(calendar_date_indicator).to_be_visible()
    expect(calendar_date_indicator).to_have_text("05.01.")

    begin_input.fill("06:00")
    expect(calendar_date_indicator).not_to_be_visible()


def test_create_entry_end_time_info_indicator(page: Page, reset_database: None):
    actions.login(page, 1, "orga")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    page.get_by_role("link", name="Eintrag hinzufügen").click()

    expect(page).to_have_title(re.compile(r"Neuer Eintrag"))
    date_input = page.get_by_role("combobox", name="Tag")
    begin_input = page.get_by_role("textbox", name="Beginn")
    duration_input = page.get_by_role("textbox", name="Dauer")
    end_time_indicator = duration_input.locator("../..").locator("#endTimeInfo")

    date_input.select_option("03.01. (Fr)")
    begin_input.fill("08:00")
    duration_input.fill("00:00")
    expect(end_time_indicator).to_be_visible()
    expect(end_time_indicator).to_have_text("Ende: 08:00")

    begin_input.fill("02:00")
    expect(end_time_indicator).to_have_text("Ende: 04.01. 02:00")
    duration_input.fill("02:20")
    expect(end_time_indicator).to_have_text("Ende: 04.01. 04:20")
    duration_input.fill("02:20:25")
    expect(end_time_indicator).to_have_text("Ende: 04.01. 04:20:25")
    date_input.select_option("04.01. (Sa)")
    expect(end_time_indicator).to_have_text("Ende: 05.01. 04:20:25")

    begin_input.fill("16:00")
    expect(end_time_indicator).to_have_text("Ende: 18:20:25")

    duration_input.fill("abc")
    expect(end_time_indicator).to_have_text("Ende: ???")

# TODO test parallel entries
