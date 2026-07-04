import dataclasses
import datetime
import re
import time

from playwright.sync_api import Browser, Page, expect

from . import actions, data, helpers


def test_submit_entry_workflow_with_prior_review(browser: Browser, reset_database: None) -> None:
    orga_context = browser.new_context()
    orga_page = orga_context.new_page()
    actions.login(orga_page, 1, "admin")
    actions.enable_entry_submission(orga_page, False)

    user_context = browser.new_context()
    user_page = user_context.new_page()
    actions.login(user_page, 1, "user")

    expect(user_page).to_have_title(re.compile(r"06\.01\."))
    user_page.get_by_role("link", name="Eintrag einreichen").click()

    expect(user_page).to_have_title(re.compile(r"Eintrag einreichen"))
    expect(user_page.get_by_role("heading", name="Eintrag einreichen")).to_be_visible()

    user_page.get_by_role("textbox", name="Titel der KüA").fill("Drachenfliegen leicht gemacht")
    user_page.get_by_role("textbox", name="von wem?").fill("Max Mustermann")
    user_page.get_by_role("button", name="Weiter").click()

    user_page.get_by_role("textbox", name="Beginn").fill("13:00")
    user_page.get_by_role("textbox", name="Dauer").fill("1,5")
    user_page.get_by_role("textbox", name="Kommentar zur Zeit").fill("Direkt nach dem Mittagessen")

    user_page.get_by_role("button", name="Weiter").click()
    user_page.get_by_role("textbox", name="Kommentar / Kurze Beschreibung").fill("wir lassen Drachen steigen")
    user_page.get_by_role("textbox", name="Ausführliche Beschreibung").fill(
        """Wir bauen Drachen und lassen sie steigen.

Für das Material müssen von jedem Teilnehmer an der KüA **5€** bezahlt werden.
        """
    )
    user_page.get_by_role("button", name="Weiter").click()

    user_page.get_by_role("textbox", name="Hinweise für die Orgas").fill("Wir brauchen wirklich die Pelikanhalle!")
    expect(user_page.get_by_role("document")).to_contain_text(
        "Nach dem Einreichen wird der Eintrag zunächst den Orgas zur Überprüfung angezeigt. Erst mit deren Bestätigung "
        "wird der Eintrag im KüA-Plan veröffentlicht."
    )
    user_page.get_by_role("checkbox", name="Ich habe die Vorschau geprüft").check()
    user_page.get_by_role("button", name="Einreichen").click()
    success_alert = user_page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    expect(success_alert).to_contain_text("wird von den Orgas geprüft")
    success_alert.get_by_role("button", name="Close").click()

    expect(user_page).to_have_title(re.compile(r"06\.01\."))
    expect(user_page.get_by_role("document")).not_to_contain_text("Drachenfliegen leicht gemacht")

    orga_page.reload()
    review_area_button = orga_page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Prüfen")
    expect(review_area_button).to_contain_text("1")
    review_area_button.click()
    expect(orga_page).to_have_title(re.compile("Zu prüfende Einträge"))
    row = helpers.get_table_row_by_column_value(orga_page, "Was?", "Drachenfliegen leicht gemacht")
    expect(row).to_contain_text("Wir brauchen wirklich die Pelikanhalle!")
    row.get_by_role("link", name="Eintrag bearbeiten").click()
    expect(orga_page.get_by_role("textbox", name="Orga-interner Kommentar")).to_contain_text(
        "Wir brauchen wirklich die Pelikanhalle!"
    )
    orga_page.locator('label:has-text("Veröffentlichen")').click()
    orga_page.get_by_role("button", name="Speichern").click()
    actions.check_success_toast(orga_page)
    expect(review_area_button).not_to_contain_text("1")
    review_area_button.click()
    expect(orga_page).to_have_title(re.compile("Zu prüfende Einträge"))
    expect(user_page.get_by_role("document")).not_to_contain_text("Drachenfliegen leicht gemacht")

    user_page.reload()
    main_table = user_page.get_by_role("table")
    expect(main_table.get_by_role("row")).to_have_count(2)
    row = main_table.get_by_role("row").nth(1)
    expect(row.get_by_role("cell").nth(0)).to_contain_text("Drachenfliegen leicht gemacht")
    expect(row.get_by_role("cell").nth(0)).to_contain_text("wir lassen Drachen steigen")
    expect(row.get_by_role("cell").nth(1)).to_contain_text("13:00 – 14:30")
    expect(row.get_by_role("cell").nth(3)).to_contain_text("Max Mustermann")
    expect(user_page.get_by_role("document")).not_to_contain_text("Wir brauchen wirklich die Pelikanhalle!")


def test_submit_entry_form_with_all_fields_and_preview(browser: Browser, page: Page, reset_database: None) -> None:
    orga_context = browser.new_context()
    orga_page = orga_context.new_page()
    actions.login(orga_page, 1, "admin")
    actions.enable_entry_submission(orga_page, True)
    actions.add_category(orga_page, data.CATEGORY_SPORT)
    actions.add_room(orga_page, data.ROOM_SPORTPLAETZE)
    actions.add_room(orga_page, data.ROOM_PELIKANHALLE)

    actions.login(page, 1, "user")
    expect(page).to_have_title(re.compile(r"06\.01\."))

    page.get_by_role("link", name="Eintrag einreichen").click()
    expect(page).to_have_title(re.compile(r"Eintrag einreichen"))
    expect(page.get_by_role("heading", name="Eintrag einreichen")).to_be_visible()

    page.get_by_role("textbox", name="Titel der KüA").fill("Drachenfliegen leicht gemacht")
    page.get_by_role("textbox", name="von wem?").fill("Max Mustermann")
    page.get_by_role("combobox", name="Kategorie").select_option(label="Sport")
    page.get_by_role("button", name="Weiter").click()

    page.get_by_role("combobox", name="Tag").select_option(label="05.01. (So)")
    page.get_by_role("textbox", name="Beginn").fill("13:00")
    page.get_by_role("textbox", name="Dauer").fill("1,5")
    page.get_by_role("textbox", name="Kommentar zur Zeit").fill("Direkt nach dem Mittagessen")
    page.get_by_role("combobox", name="Orte").fill("Pelikanhalle")
    page.get_by_role("option", name="Pelikanhalle", exact=True).click()
    page.get_by_role("combobox", name="Orte").fill("Sportplätze")
    page.get_by_role("option", name="Sportplätze", exact=True).click()
    page.get_by_role("textbox", name="Kommentar zum Ort").fill("bei schlechtem Wetter sind wir in der Halle")

    page.get_by_role("button", name="Weiter").click()
    page.get_by_role("textbox", name="Kommentar / Kurze Beschreibung").fill("wir lassen Drachen steigen")
    page.get_by_role("textbox", name="Ausführliche Beschreibung").fill(
        """Wir bauen Drachen und lassen sie steigen.

Für das Material müssen von jedem Teilnehmer an der KüA **5€** bezahlt werden.
        """
    )
    page.get_by_role("button", name="Weiter").click()

    time.sleep(0.4)  # Give the browser some time to fetch the Markdown description preview
    preview_section = page.get_by_role("region", name="Vorschau")
    preview_table = preview_section.get_by_role("table")
    expect(preview_table.get_by_role("row")).to_have_count(2)
    row = preview_table.get_by_role("row").nth(1)
    expect(row.get_by_role("cell").nth(0)).to_contain_text("Drachenfliegen leicht gemacht")
    assert helpers.is_text_colored(row.get_by_text("Drachenfliegen"))
    # TODO check for category icon when implemented
    expect(row.get_by_role("cell").nth(0)).to_contain_text("wir lassen Drachen steigen")
    expect(row.get_by_role("cell").nth(1)).to_contain_text("13:00 – 14:30")
    expect(row.get_by_role("cell").nth(1)).to_contain_text("Direkt nach dem Mittagessen")
    expect(row.get_by_role("cell").nth(2)).to_contain_text("Pelikanhalle, Sportplätze")
    expect(row.get_by_role("cell").nth(2)).to_contain_text("bei schlechtem Wetter sind wir in der Halle")
    expect(row.get_by_role("cell").nth(3)).to_contain_text("Max Mustermann")
    expect(preview_section).to_contain_text("Für das Material müssen von jedem Teilnehmer")
    assert helpers.is_text_bold(preview_section.get_by_text("5€"))

    page.get_by_role("checkbox", name="Ich habe die Vorschau geprüft").check()
    page.get_by_role("button", name="Veröffentlichen").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    expect(success_alert).to_contain_text("veröffentlicht.")
    success_alert.get_by_role("button", name="Close").click()

    expect(page).to_have_title(re.compile(r"05\.01\."))
    main_table = page.get_by_role("table")
    expect(main_table.get_by_role("row")).to_have_count(2)
    row = main_table.get_by_role("row").nth(1)
    expect(row.get_by_role("cell").nth(0)).to_contain_text("Drachenfliegen leicht gemacht")
    expect(row.get_by_role("cell").nth(0)).to_contain_text("⚽")
    expect(row.get_by_role("cell").nth(0)).to_contain_text("wir lassen Drachen steigen")
    expect(row.get_by_role("cell").nth(1)).to_contain_text("13:00 – 14:30")
    expect(row.get_by_role("cell").nth(1)).to_contain_text("Direkt nach dem Mittagessen")
    expect(row.get_by_role("cell").nth(2)).to_contain_text("Pelikanhalle, Sportplätze")
    expect(row.get_by_role("cell").nth(2)).to_contain_text("bei schlechtem Wetter sind wir in der Halle")
    expect(row.get_by_role("cell").nth(3)).to_contain_text("Max Mustermann")
    expect(page.get_by_role("document")).to_contain_text("Für das Material müssen von jedem Teilnehmer")
    assert helpers.is_text_bold(page.get_by_text("5€"))


def test_submit_entry_form_tab_navigation(browser: Browser, page: Page, reset_database: None) -> None:
    orga_context = browser.new_context()
    orga_page = orga_context.new_page()
    actions.login(orga_page, 1, "admin")
    actions.enable_entry_submission(orga_page, True)

    actions.login(page, 1, "user")
    page.get_by_role("link", name="Eintrag einreichen").click()
    expect(page).to_have_title(re.compile(r"Eintrag einreichen"))

    title_input = page.get_by_role("textbox", name="Titel der KüA")
    day_input = page.get_by_role("combobox", name="Tag")
    description_input = page.get_by_role("textbox", name="Ausführliche Beschreibung")
    orga_comment_input = page.get_by_role("textbox", name="Hinweise für die Orgas")

    expect(title_input).to_be_visible()
    expect(day_input).not_to_be_visible()

    page.get_by_role("tab", name="Zeit & Ort").click()
    expect(day_input).to_be_visible()

    page.go_back()
    expect(title_input).to_be_visible()

    page.get_by_role("button", name="Weiter").click()
    page.get_by_role("button", name="Weiter").click()
    expect(description_input).to_be_visible()

    page.go_back()
    expect(day_input).to_be_visible()

    page.go_forward()
    expect(description_input).to_be_visible()

    page.reload()
    expect(description_input).to_be_visible()

    page.get_by_role("button", name="Zurück").click()
    expect(day_input).to_be_visible()

    page.get_by_role("tab", name="Vorschau").click()
    expect(orga_comment_input).to_be_visible()

    page.go_back()
    expect(day_input).to_be_visible()

    page.go_forward()
    expect(orga_comment_input).to_be_visible()


def test_submit_entry_date_info_indicator(page: Page, browser: Browser, reset_database: None) -> None:
    orga_context = browser.new_context()
    orga_page = orga_context.new_page()
    actions.login(orga_page, 1, "admin")
    actions.enable_entry_submission(orga_page, True)

    actions.login(page, 1, "user")
    page.get_by_role("link", name="Eintrag einreichen").click()
    expect(page).to_have_title(re.compile(r"Eintrag einreichen"))
    page.get_by_role("tab", name="Zeit & Ort").click()
    date_input = page.get_by_role("combobox", name="Tag")
    begin_input = page.get_by_role("textbox", name="Beginn")

    date_input.select_option("03.01. (Fr)")
    begin_input.fill("02:00")
    calendar_date_indicator = begin_input.locator("..").locator("#calendarDateInfo")
    expect(calendar_date_indicator).to_be_visible()
    expect(calendar_date_indicator).to_contain_text("Beginn am nächsten Kalendertag")
    expect(calendar_date_indicator).to_contain_text("04.01. 02:00")
    assert helpers.is_text_colored(calendar_date_indicator)

    date_input.select_option("04.01. (Sa)")
    expect(calendar_date_indicator).to_be_visible()
    expect(calendar_date_indicator).to_contain_text("05.01.")

    begin_input.fill("06:00")
    expect(calendar_date_indicator).not_to_be_visible()


def test_submit_entry_parallel_entries(page: Page, browser: Browser, reset_database: None) -> None:
    orga_context = browser.new_context()
    orga_page = orga_context.new_page()
    actions.login(orga_page, 1, "admin")
    actions.enable_entry_submission(orga_page, True)
    actions.add_category(orga_page, data.CATEGORY_SPORT)
    actions.add_room(orga_page, data.ROOM_SPORTPLAETZE)
    actions.add_room(orga_page, data.ROOM_PELIKANHALLE)
    actions.add_room(orga_page, data.ROOM_SEMINARRAUM)
    actions.add_entry(
        orga_page,
        dataclasses.replace(
            data.ENTRY_BEACH_VOLLEYBALL,
            day=datetime.date(2025, 1, 1),
            begin=datetime.time(18, 30),
            duration=datetime.timedelta(hours=1, minutes=20, seconds=20),
            time_comment="Schnell noch vor dem Plenum :)",
        ),
    )
    actions.add_entry(orga_page, data.ENTRY_BEGRUESSUNGSPLENUM)  # from 20:00
    actions.add_entry(orga_page, data.ENTRY_PLENUMSVORBEREITUNG)  # 19:30 – 20:00
    actions.add_entry(
        orga_page, dataclasses.replace(data.ENTRY_LOREM_IPSUM, day=datetime.date(2025, 1, 1)), as_draft=True
    )  # from 14:00

    actions.login(page, 1, "user")
    page.get_by_role("link", name="Eintrag einreichen").click()
    expect(page).to_have_title(re.compile(r"Eintrag einreichen"))
    page.get_by_role("tab", name="Zeit & Ort").click()
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
    # Parallel to Lorem Ipsum entry, which is not public yet
    begin_input.fill("14:00")
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
    assert helpers.is_text_bold(items[1].get_by_text("Seminarraum Pelikanhalle"))
    assert helpers.is_text_colored(items[1].get_by_text("Seminarraum Pelikanhalle"))
    expect(items[2]).to_contain_text("Beach-Volleyball")  # no conflict
    assert not helpers.is_text_colored(items[2].get_by_text("Sportplätze"))

    duration_input.fill("test")
    expect(parallel_entries_overlays).to_be_visible()
    expect(parallel_entries_overlays).to_contain_text("Ungültige Eingabedaten")


def test_submit_entry_workflow_with_review_after_publishing(browser: Browser, reset_database: None) -> None:
    orga_context = browser.new_context()
    orga_page = orga_context.new_page()
    actions.login(orga_page, 1, "admin")
    actions.enable_entry_submission(orga_page, True)

    user_context = browser.new_context()
    user_page = user_context.new_page()
    actions.login(user_page, 1, "user")

    expect(user_page).to_have_title(re.compile(r"06\.01\."))
    user_page.get_by_role("link", name="Eintrag einreichen").click()

    expect(user_page).to_have_title(re.compile(r"Eintrag einreichen"))
    expect(user_page.get_by_role("heading", name="Eintrag einreichen")).to_be_visible()

    user_page.get_by_role("textbox", name="Titel der KüA").fill("Drachenfliegen leicht gemacht")
    user_page.get_by_role("textbox", name="von wem?").fill("Max Mustermann")
    user_page.get_by_role("button", name="Weiter").click()

    user_page.get_by_role("textbox", name="Beginn").fill("13:00")
    user_page.get_by_role("textbox", name="Dauer").fill("1,5")
    user_page.get_by_role("textbox", name="Kommentar zur Zeit").fill("Direkt nach dem Mittagessen")

    user_page.get_by_role("button", name="Weiter").click()
    user_page.get_by_role("textbox", name="Kommentar / Kurze Beschreibung").fill("wir lassen Drachen steigen")
    user_page.get_by_role("textbox", name="Ausführliche Beschreibung").fill(
        """Wir bauen Drachen und lassen sie steigen.

Für das Material müssen von jedem Teilnehmer an der KüA **5€** bezahlt werden.
        """
    )
    user_page.get_by_role("button", name="Weiter").click()

    # Direct publishing should be the default option
    user_page.get_by_role("textbox", name="Hinweise für die Orgas").fill("Wir brauchen wirklich die Pelikanhalle!")
    expect(user_page.get_by_role("checkbox", name="Direkt veröffentlichen")).to_be_checked()
    user_page.get_by_role("checkbox", name="Ich habe die Vorschau geprüft").check()
    user_page.get_by_role("button", name="Veröffentlichen").click()
    success_alert = user_page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    expect(success_alert).to_contain_text("veröffentlicht.")
    success_alert.get_by_role("button", name="Close").click()

    expect(user_page).to_have_title(re.compile(r"06\.01\."))
    main_table = user_page.get_by_role("table")
    expect(main_table.get_by_role("row")).to_have_count(2)
    row = main_table.get_by_role("row").nth(1)
    expect(row.get_by_role("cell").nth(0)).to_contain_text("Drachenfliegen leicht gemacht")
    expect(row.get_by_role("cell").nth(0)).to_contain_text("wir lassen Drachen steigen")
    expect(row.get_by_role("cell").nth(1)).to_contain_text("13:00 – 14:30")
    expect(row.get_by_role("cell").nth(3)).to_contain_text("Max Mustermann")
    expect(user_page.get_by_role("document")).not_to_contain_text("Wir brauchen wirklich die Pelikanhalle!")

    orga_page.reload()
    review_area_button = orga_page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Prüfen")
    expect(review_area_button).to_contain_text("1")
    review_area_button.click()
    expect(orga_page).to_have_title(re.compile("Zu prüfende Einträge"))
    row = helpers.get_table_row_by_column_value(orga_page, "Was?", "Drachenfliegen leicht gemacht")
    expect(row).to_contain_text("Wir brauchen wirklich die Pelikanhalle!")
    row.get_by_role("link", name="Eintrag bearbeiten").click()
    expect(orga_page.get_by_role("textbox", name="Orga-interner Kommentar")).to_contain_text(
        "Wir brauchen wirklich die Pelikanhalle!"
    )
    orga_page.locator('label:has-text("Bestätigen")').click()
    orga_page.get_by_role("button", name="Speichern").click()
    actions.check_success_toast(orga_page)
    expect(review_area_button).not_to_contain_text("1")
    review_area_button.click()
    expect(orga_page).to_have_title(re.compile("Zu prüfende Einträge"))
    expect(orga_page.get_by_role("document")).not_to_contain_text("Drachenfliegen leicht gemacht")


# TODO test that "official" categories are not selectable

# TODO test going to "Eintrag einreichen" page or manually POSTing entry submission does not work when disabled
