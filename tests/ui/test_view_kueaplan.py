import re

from playwright.sync_api import Page, expect

from . import actions, data, helpers


def test_empty_day_notification(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "user")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    expect(page.get_by_role("heading", name="KüA-Plan")).to_be_visible()
    expect(page.get_by_text("keine KüAs geplant")).to_be_visible()


def test_empty_category_notification(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "user")

    page.get_by_role("navigation").get_by_role("link", name="Kategorien").click()
    expect(page.get_by_role("heading", name="KüA-Kategorien")).to_be_visible()

    page.get_by_role("link", name="Default").click()
    expect(page.get_by_role("heading", name="KüA-Plan")).to_be_visible()
    expect(page.get_by_text(re.compile(r"keine KüAs in der Kategorie .*? geplant"))).to_be_visible()


def test_main_list_entry(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_category(page, data.CATEGORY_SPORT)
    actions.add_room(page, data.ROOM_SPORTPLAETZE)
    actions.add_entry(page, data.ENTRY_BEACH_VOLLEYBALL)

    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Sa 04.01.").click()
    row = helpers.get_table_row_by_column_value(page, "Was?", "Beach-Volleyball")
    expect(row).to_be_visible()

    category_icon = row.get_by_text("⚽")
    expect(category_icon).to_be_visible()
    expect(category_icon).to_have_attribute("title", "Kategorie Sport")

    title_col = helpers.get_table_cell_by_header(row, "Was?")
    comment = title_col.get_by_text("bitte Bälle und Musik mitbringen")
    expect(comment).to_be_visible()
    helpers.assert_small_font(comment)

    time_col = helpers.get_table_cell_by_header(row, "Wann?")
    expect(time_col).to_contain_text("13:30 – 15:00")
    person_col = helpers.get_table_cell_by_header(row, "Von wem?")
    expect(person_col).to_contain_text("Fabienne Wagener")
    room_col = helpers.get_table_cell_by_header(row, "Wo?")
    expect(room_col).to_contain_text("Sportplätze")
    room_comment = room_col.get_by_text("Beach-Volleyball-Feld")
    expect(room_comment).to_be_visible()
    helpers.assert_small_font(room_comment)
