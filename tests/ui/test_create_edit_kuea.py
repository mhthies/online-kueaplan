import dataclasses
import datetime
import re
import time

import playwright.sync_api
from playwright.sync_api import Browser, Page, expect

from . import actions, data, helpers
from .data import (
    CATEGORY_SPORT,
    ENTRY_BEACH_VOLLEYBALL,
    ENTRY_BEGRUESSUNGSPLENUM,
    ENTRY_PLENUMSVORBEREITUNG,
    ENTRY_SONNENAUFGANG_WANDERUNG,
    ROOM_PELIKANHALLE,
    ROOM_SEMINARRAUM,
    ROOM_SPORTPLAETZE,
)
from .helpers import is_text_bold, is_text_colored


def test_create_entry(page: Page, reset_database: None) -> None:
    # Note: This test is somehow redundant to test_main_list_entry_attributes(), but also tests some different aspects
    #   of the add_entry form.
    actions.login(page, 1, "orga")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    page.get_by_role("link", name="Neuer Eintrag").click()

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
        """
    )
    page.get_by_role("button", name="Erstellen").click()
    expect(page).to_have_title(re.compile(r"06\.01\."))

    main_table = page.get_by_role("table")
    expect(main_table.get_by_role("row")).to_have_count(2)
    row = main_table.get_by_role("row").nth(1)
    expect(row.get_by_role("cell").nth(0)).to_contain_text("Drachenfliegen leicht gemacht")
    expect(row.get_by_role("cell").nth(0)).to_contain_text("wir lassen Drachen steigen")
    expect(row.get_by_role("cell").nth(1)).to_contain_text("13:00 – 14:30")
    expect(row.get_by_role("cell").nth(3)).to_contain_text("Max Mustermann")


def test_create_entry_prefilled_date(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    page.get_by_role("button", name="Datum").click()  # open date dropdown
    page.get_by_role("link", name="Do 02.01.").click()
    expect(page).to_have_title(re.compile(r"02\.01\."))

    page.get_by_role("link", name="Neuer Eintrag").click()
    expect(page.get_by_role("combobox", name="Tag")).to_have_value("2025-01-02")


def test_create_entry_validation_error_duration(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    page.get_by_role("link", name="Neuer Eintrag").click()

    expect(page).to_have_title(re.compile(r"Neuer Eintrag"))
    expect(page.get_by_role("heading", name="Neuer Eintrag")).to_be_visible()

    page.get_by_role("textbox", name="Titel").fill("Drachenfliegen leicht gemacht")
    page.get_by_role("textbox", name="von wem?").fill("Max Mustermann")
    page.get_by_role("textbox", name="Beginn").fill("13:00")
    page.get_by_role("textbox", name="Dauer").fill("1:")
    page.get_by_role("button", name="Erstellen").click()

    error_alert = page.get_by_role("alert").filter(has_text="Eingegebene Daten sind ungültig")
    expect(error_alert).to_be_visible()
    expect(page).to_have_title(re.compile(r"Neuer Eintrag"))
    duration_input = page.get_by_role("textbox", name="Dauer")
    # parent of input field has 'is-invalid' class for red marker
    expect(duration_input.locator("..")).to_have_class(re.compile(r"(^|\s)is-invalid(\s|$)"))
    # error text within form row (parent of input group)
    expect(duration_input.locator("../..")).to_have_text(re.compile(r"Keine gültige Dauer"))


def test_create_entry_date_info_indicator(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    page.get_by_role("link", name="Neuer Eintrag").click()

    expect(page).to_have_title(re.compile(r"Neuer Eintrag"))
    date_input = page.get_by_role("combobox", name="Tag")
    begin_input = page.get_by_role("textbox", name="Beginn")

    date_input.select_option("03.01. (Fr)")
    begin_input.fill("02:00")
    calendar_date_indicator = begin_input.locator("..").locator("#calendarDateInfo")
    expect(calendar_date_indicator).to_be_visible()
    expect(calendar_date_indicator).to_have_text("Kalendertag: 04.01.")
    expect(calendar_date_indicator).to_have_css("color", "rgb(13, 202, 240)")

    date_input.select_option("04.01. (Sa)")
    expect(calendar_date_indicator).to_be_visible()
    expect(calendar_date_indicator).to_have_text("Kalendertag: 05.01.")

    begin_input.fill("06:00")
    expect(calendar_date_indicator).not_to_be_visible()


def test_create_entry_end_time_info_indicator(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    page.get_by_role("link", name="Neuer Eintrag").click()

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


def test_create_entry_parallel_entries(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_category(page, CATEGORY_SPORT)
    actions.add_room(page, ROOM_SPORTPLAETZE)
    actions.add_room(page, ROOM_PELIKANHALLE)
    actions.add_room(page, ROOM_SEMINARRAUM)
    actions.add_entry(
        page,
        dataclasses.replace(
            ENTRY_BEACH_VOLLEYBALL,
            day=datetime.date(2025, 1, 1),
            begin=datetime.time(18, 30),
            duration=datetime.timedelta(hours=1, minutes=20, seconds=20),
            time_comment="Schnell noch vor dem Plenum :)",
        ),
    )
    actions.add_entry(page, ENTRY_BEGRUESSUNGSPLENUM)  # from 20:00
    actions.add_entry(page, ENTRY_PLENUMSVORBEREITUNG)  # 19:30 – 20:00

    expect(page).to_have_title(re.compile(r"01\.01\."))
    page.get_by_role("link", name="Neuer Eintrag").click()

    expect(page).to_have_title(re.compile(r"Neuer Eintrag"))
    page.get_by_role("combobox", name="Tag").select_option("01.01. (Mi)")
    page.get_by_role("combobox", name="Orte").fill("Seminarraum Pelikanhalle")
    page.get_by_role("option", name="Seminarraum Pelikanhalle").click()

    begin_input = page.get_by_role("textbox", name="Beginn")
    duration_input = page.get_by_role("textbox", name="Dauer")
    parallel_entries_box = page.get_by_role("complementary", name="Parallele Einträge")
    parallel_entries_overlays = parallel_entries_box.get_by_role("status")

    begin_input.fill("08:00")
    duration_input.fill("1:00")
    expect(parallel_entries_overlays).not_to_be_visible()
    expect(parallel_entries_box).to_contain_text("Keine parallelen Einträge")

    begin_input.fill("18:00")
    expect(parallel_entries_overlays).not_to_be_visible()
    expect(parallel_entries_box).not_to_contain_text("Keine parallelen Einträge")
    list_item = parallel_entries_box.get_by_role("listitem")
    expect(list_item).to_have_count(1)
    expect(list_item).to_contain_text("Beach-Volleyball")
    expect(list_item).to_contain_text("18:30 – 19:50")

    begin_input.fill("18:30")
    expect(parallel_entries_overlays).not_to_be_visible()
    expect(list_item).to_have_count(1)

    begin_input.fill("19:30")
    expect(list_item).to_have_count(3)
    # Items should be ordered from most problematic to least problematic (and then by time)
    items = list_item.all()
    expect(items[0]).to_contain_text("Begrüßungsplenum")  # exclusive
    expect(items[0]).to_contain_text("exklusiv")
    expect(items[1]).to_contain_text("Plenums-Vorbereitung")  # room conflict
    expect(items[1]).to_contain_text("Pelikanhalle, Seminarraum Pelikanhalle")
    assert is_text_bold(items[1].get_by_text("Seminarraum Pelikanhalle"))
    assert is_text_colored(items[1].get_by_text("Seminarraum Pelikanhalle"))
    expect(items[2]).to_contain_text("Beach-Volleyball")  # no conflict
    assert not is_text_colored(items[2].get_by_text("Sportplätze"))

    duration_input.fill("test")
    expect(parallel_entries_overlays).to_be_visible()
    expect(parallel_entries_overlays).to_contain_text("Ungültige Eingabedaten")


def test_clone_entry(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_entry(page, ENTRY_SONNENAUFGANG_WANDERUNG)
    row = helpers.get_table_row_by_column_value(page, "Was?", "Sonnenaufgang-Wanderung")
    row.get_by_title("bearbeiten").click()
    page.get_by_role("link", name="Duplizieren").click()
    expect(page).to_have_title(re.compile("Neuer Eintrag"))
    expect(page.get_by_role("textbox", name="Titel")).to_have_value("Sonnenaufgang-Wanderung")
    expect(page.get_by_role("combobox", name="Tag")).to_have_value("2025-01-05")
    page.get_by_role("combobox", name="Tag").select_option(value="2025-01-04")
    page.get_by_role("button", name="Erstellen").click()
    expect(page.get_by_role("alert").filter(has_text="Erfolg")).to_be_visible()

    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="05.01.").click()
    # Due to the carryover to the next day of the entry, crossing the day boundary, we should see both duplicates of the
    #   entry on the 4th of January.
    expect(page.get_by_role("cell").filter(has_text="Sonnenaufgang-Wanderung")).to_have_count(2)


def test_detect_concurrent_entry_change(browser: Browser, reset_database: None) -> None:
    context1 = browser.new_context()
    page1 = context1.new_page()
    context2 = browser.new_context()
    page2 = context2.new_page()
    actions.login(page1, 1, "orga")
    actions.login(page2, 1, "orga")
    actions.add_entry(page1, data.ENTRY_AKROBATIK)

    page2.get_by_role("button", name="Datum").click()
    page2.get_by_role("link", name="Fr 03.01.").click()
    helpers.get_table_row_by_column_value(page2, "Was?", "Akrobatik").get_by_role("link", name="bearbeiten").click()
    page2.get_by_role("textbox", name="Kommentar / Kurze Beschreibung").fill(
        "Erfahrung von Akrobatik-KüAs wird vorausgesetzt"
    )

    helpers.get_table_row_by_column_value(page1, "Was?", "Akrobatik").get_by_role("link", name="bearbeiten").click()
    page1.get_by_role("textbox", name="von wem? / Ansprechpersonen").fill("Lilo Thiemann und Flavio Blume")
    page1.get_by_role("button", name="Speichern").click()
    actions.check_success_toast(page1)

    page2.get_by_role("button", name="Speichern").click()
    error_alert = page2.get_by_role("alert").filter(has_text="Fehler")
    expect(error_alert).to_be_visible()
    expect(error_alert).to_contain_text("zwischenzeitlich bearbeitet")
    expect(error_alert).to_contain_text("Bitte das Formular neu laden und die Änderung erneut durchführen")
    error_alert.get_by_role("link", name="Formular neuladen").click()

    expect(page2.get_by_role("textbox", name="von wem? / Ansprechpersonen")).to_have_value(
        "Lilo Thiemann und Flavio Blume"
    )
    page2.get_by_role("textbox", name="Kommentar / Kurze Beschreibung").fill(
        "Erfahrung von Akrobatik-KüAs wird vorausgesetzt"
    )
    page2.get_by_role("button", name="Speichern").click()

    row = helpers.get_table_row_by_column_value(page2, "Was?", "Akrobatik")
    expect(row).to_contain_text("Lilo Thiemann und Flavio Blume")
    expect(row).to_contain_text("Erfahrung von Akrobatik-KüAs wird vorausgesetzt")


def test_detect_unsaved_changes(page: Page, reset_database: None) -> None:
    dialog_shown = False

    def handle_dialog(dialog: playwright.sync_api.Dialog) -> None:
        if dialog.type == "beforeunload":
            nonlocal dialog_shown
            dialog_shown = True
        dialog.dismiss()

    page.on("dialog", handle_dialog)

    actions.login(page, 1, "orga")
    actions.add_entry(page, data.ENTRY_AKROBATIK)
    helpers.get_table_row_by_column_value(page, "Was?", "Akrobatik").get_by_role("link", name="bearbeiten").click()

    # No effective changes -> no dialog shown
    page.get_by_role("textbox", name="Kommentar zum Ort").fill("Wir laufen über's Gelände")
    page.get_by_role("textbox", name="Kommentar zur Zeit").focus()
    page.get_by_role("textbox", name="Kommentar zum Ort").fill("")
    page.get_by_role("link", name="Vorherige Termine").click()
    expect(page).to_have_title(re.compile(r"Vorherige Termine"))
    assert not dialog_shown

    # with changes: onbeforeunload dialog is shown for confirming
    page.get_by_role("link", name="Bearbeiten").click()
    page.get_by_role("textbox", name="von wem? / Ansprechpersonen").fill("Lilo Thiemann und Flavio Blume")
    page.get_by_role("link", name="Vorherige Termine").click()
    tic = time.time()
    while not dialog_shown and time.time() - tic < 5.0:
        time.sleep(0.01)
    assert dialog_shown

    # Special handling of cancel button: no dialog shown
    dialog_shown = False  # type: ignore [unreachable]  # mypy does not get the mutability of `dialog_shown`
    page.get_by_role("link", name="Abbrechen").click()
    expect(page).to_have_title(re.compile(r"03.01."))
    assert not dialog_shown
