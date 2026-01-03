import re

from playwright.sync_api import Page, expect

from . import actions, data, helpers


def test_previous_date_view_merged(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_room(page, data.ROOM_SPORTPLAETZE)
    actions.add_room(page, data.ROOM_PELIKANHALLE)
    actions.add_category(page, data.CATEGORY_SPORT)
    actions.add_entry(page, data.ENTRY_BEACH_VOLLEYBALL)

    page.get_by_role("link", name="Eintrag bearbeiten").click()
    page.get_by_role("textbox", name="Beginn").fill("14:30")
    page.get_by_role("combobox", name="Orte").press("Backspace")  # Delete "Sportplätze"
    page.get_by_role("combobox", name="Orte").fill("Pelikanhalle")
    page.get_by_role("textbox", name="Kommentar zum Ort").clear()
    page.get_by_role("option", name="Pelikanhalle").click()
    page.get_by_role("checkbox", name="Hinweis zur Verschiebung am vorherigen Termin im KüA-Plan anlegen").check()
    page.get_by_role("textbox", name="Kommentar zur Verschiebung").fill("Wegen schlechten Wetters in der Halle")
    page.get_by_role("button", name="Speichern").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()

    row = helpers.get_table_row_by_column_value(page, "Was?", "Beach-Volleyball")
    expect(row).to_be_visible()
    rooms_cell = helpers.get_table_cell_by_header(row, "Wo?")
    expect(rooms_cell).to_have_text(re.compile(r"\s*Pelikanhalle.*Zuvor geplante Orte:\s*Sportplätze\s*", re.DOTALL))
    assert helpers.is_line_through(rooms_cell.get_by_text("Sportplätze"))
    time_cell = helpers.get_table_cell_by_header(row, "Wann?")
    expect(time_cell).to_have_text(
        re.compile(r"\s*14:30\s*–\s*16:00.*Zuvor geplante Zeiten:\s*13:30\s*–\s*15:00\s*", re.DOTALL)
    )
    assert helpers.is_line_through(time_cell.get_by_text("13:30"))
    expect(row.get_by_text("Wegen schlechten Wetters in der Halle")).to_be_visible()


def test_previous_date_not_merged(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_room(page, data.ROOM_SPORTPLAETZE)
    actions.add_room(page, data.ROOM_PELIKANHALLE)
    actions.add_category(page, data.CATEGORY_SPORT)
    actions.add_entry(page, data.ENTRY_BEACH_VOLLEYBALL)
    actions.add_entry(page, data.ENTRY_LOREM_IPSUM)

    row = helpers.get_table_row_by_column_value(page, "Was?", "Beach-Volleyball")
    row.get_by_role("link", name="Eintrag bearbeiten").click()
    page.get_by_role("textbox", name="Beginn").fill("14:30")
    page.get_by_role("combobox", name="Orte").press("Backspace")  # Delete "Sportplätze"
    page.get_by_role("combobox", name="Orte").fill("Pelikanhalle")
    page.get_by_role("textbox", name="Kommentar zum Ort").clear()
    page.get_by_role("option", name="Pelikanhalle").click()
    page.get_by_role("checkbox", name="Hinweis zur Verschiebung am vorherigen Termin im KüA-Plan anlegen").check()
    page.get_by_role("textbox", name="Kommentar zur Verschiebung").fill("Wegen schlechten Wetters in der Halle")
    page.get_by_role("button", name="Speichern").click()
    success_alert = page.get_by_role("alert").filter(has_text="Erfolg")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()

    row = helpers.get_table_row_by_column_value(page, "Was?", "Beach-Volleyball")
    expect(row).to_have_count(2)
    entry_anchor = row.last.locator("xpath=(./td)[1]").get_attribute("id")
    move_link = row.first.get_by_role("link", name="14:30")
    expect(move_link).to_be_visible()
    expect(move_link).to_have_attribute("href", re.compile(rf".*#{re.escape(entry_anchor)}"))

    room = row.first.get_by_text("Sportplätze")
    expect(room).to_be_visible()
    assert helpers.is_line_through(room)
    expect(helpers.get_table_cell_by_header(row.first, "Wo?")).not_to_contain_text("Pelikanhalle")
    time = row.first.get_by_text("13:30")
    expect(time).to_be_visible()
    assert helpers.is_line_through(time)
    expect(helpers.get_table_cell_by_header(row.first, "Wann?")).not_to_contain_text("14:30")
    move_comment = row.first.get_by_text("Wegen schlechten Wetters in der Halle")
    expect(move_comment).to_be_visible()
    assert not helpers.is_line_through(move_comment)

    expect(row.last).to_contain_text("Pelikanhalle")
    expect(row.last).not_to_contain_text("Sportplätze")
    expect(row.last).to_contain_text("14:30")
    expect(row.last).not_to_contain_text("13:30")
    expect(row.last).not_to_contain_text("Wegen schlechten Wetters in der Halle")
