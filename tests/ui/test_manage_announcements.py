import datetime

from playwright.sync_api import Page, expect

from tests.ui import actions, data, helpers


def test_simple_announcement(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_room(page, data.ROOM_PELIKANHALLE)
    actions.add_room(page, data.ROOM_SPORTPLAETZE)
    actions.add_announcement(page, data.ANNOUNCEMENT_SPORTPLATZ_NASS)

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Sa 04.01.").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung")
    expect(bekanntmachung).to_be_visible()
    expect(bekanntmachung).to_contain_text("Warnung")
    expect(bekanntmachung).to_contain_text("Achtung: Auf dem Sportplatz ist es nass und rutschig. Bitte aufpassen.")
    assert helpers.is_text_colored(bekanntmachung.get_by_text("Warnung"))

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Mo 06.01.").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).to_be_visible()

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Do 02.01.").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).not_to_be_visible()

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Orte").click()
    page.get_by_role("link", name="Sportplätze").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).to_be_visible()

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Orte").click()
    page.get_by_role("link", name="Pelikanhalle").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).not_to_be_visible()

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Kategorien").click()
    page.get_by_role("link", name="Default").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).not_to_be_visible()


def test_edit_announcement(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_room(page, data.ROOM_SPORTPLAETZE)
    actions.add_announcement(page, data.ANNOUNCEMENT_SPORTPLATZ_NASS)

    announcement_row = helpers.get_table_row_by_column_value(page, "Inhalt", "nass und rutschig")
    announcement_row.get_by_role("link", name="Bearbeiten").click()
    page.get_by_role("combobox", name="Typ").select_option(label="Information")
    page.get_by_role("textbox", name="Text").fill(
        "Auf dem Sportplatz ist es nass und rutschig.\n\nIn der Pelikanhalle ist es nicht nass und rutschig."
    )
    page.get_by_role("combobox", name="ab Datum").select_option(label="Anfang")
    page.get_by_role("combobox", name="bis Datum").select_option(label="03.01.")
    page.get_by_role("checkbox", name="Anzeigen im KüA-Plan nach Kategorie").check()
    # Entering no category = show for all categories
    page.get_by_role("checkbox", name="Anzeigen im KüA-Plan nach Raum").uncheck()
    page.get_by_role("button", name="Speichern").click()
    actions.check_success_toast(page)

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Fr 03.01.").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung")
    expect(bekanntmachung).to_be_visible()
    expect(bekanntmachung).to_contain_text("Information")
    expect(bekanntmachung.get_by_role("paragraph")).to_have_count(2)

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Mi 01.01.").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).to_be_visible()

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Sa 04.01.").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).not_to_be_visible()

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Orte").click()
    page.get_by_role("link", name="Sportplätze").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).not_to_be_visible()

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Kategorien").click()
    page.get_by_role("link", name="Default").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).to_be_visible()


def test_hide_announcement(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_room(page, data.ROOM_SPORTPLAETZE)
    actions.add_announcement(page, data.ANNOUNCEMENT_SPORTPLATZ_NASS)

    announcement_row = helpers.get_table_row_by_column_value(page, "Inhalt", "nass und rutschig")
    announcement_row.get_by_role("link", name="Löschen").click()
    page.get_by_role("button", name="Ausblenden").click()
    actions.check_success_toast(page)

    announcement_row = helpers.get_table_row_by_column_value(page, "Inhalt", "nass und rutschig")
    assert helpers.is_line_through(announcement_row.get_by_text("nass und rutschig"))
    announcement_row.get_by_role("link", name="Bearbeiten").click()
    expect(page.get_by_role("checkbox", name="Anzeigen im KüA-Plan nach Raum")).not_to_be_checked()
    expect(page.get_by_role("checkbox", name="Anzeigen im KüA-Plan nach Datum")).not_to_be_checked()

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Sa 04.01.").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).not_to_be_visible()

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Orte").click()
    page.get_by_role("link", name="Sportplätze").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).not_to_be_visible()


def test_delete_announcement(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_room(page, data.ROOM_SPORTPLAETZE)
    actions.add_announcement(page, data.ANNOUNCEMENT_SPORTPLATZ_NASS)

    announcement_row = helpers.get_table_row_by_column_value(page, "Inhalt", "nass und rutschig")
    announcement_row.get_by_role("link", name="Löschen").click()
    page.get_by_role("button", name="Löschen").click()
    actions.check_success_toast(page)

    announcement_row = helpers.get_table_row_by_column_value(page, "Inhalt", "nass und rutschig")
    expect(announcement_row).not_to_be_visible()

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Sa 04.01.").click()
    bekanntmachung = page.get_by_role("complementary", name="Bekanntmachung").filter(has_text="nass und rutschig")
    expect(bekanntmachung).not_to_be_visible()


def test_announcement_sorting(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_room(page, data.ROOM_SPORTPLAETZE)
    actions.add_announcement(page, data.ANNOUNCEMENT_SPORTPLATZ_NASS)
    actions.add_announcement(
        page,
        actions.Announcement(
            "Wegen einer anderen Gruppe ist abends mit lauter Musik zu rechnen.",
            actions.AnnouncementType.INFO,
            sort_key=-10,
            show_with_days=True,
            begin_date=datetime.date(2025, 1, 2),
            end_date=datetime.date(2025, 1, 4),
            show_with_rooms=True,
            rooms=["Sportplätze"],
        ),
    )
    actions.add_announcement(
        page,
        actions.Announcement(
            "Bitte hinterlasst alle Räume aufgeräumt.",
            actions.AnnouncementType.INFO,
            sort_key=10,
            show_with_days=True,
            show_with_rooms=True,
        ),
    )

    expect(page.get_by_role("row")).to_contain_text(["lauter Musik", "nass und rutschig", "aufgeräumt"])

    page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Sa 04.01.").click()
    expect(page.get_by_role("complementary", name="Bekanntmachung")).to_contain_text(
        ["lauter Musik", "nass und rutschig", "aufgeräumt"]
    )
    expect(page.get_by_role("complementary", name="Bekanntmachung")).to_contain_text(
        ["Information", "Warnung", "Information"]
    )
