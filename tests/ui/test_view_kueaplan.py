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


def test_main_list_entry_attributes(page: Page, reset_database: None) -> None:
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

    page.get_by_role("link", name="Orte").click()
    page.get_by_role("link", name="Sportplätze").click()
    row = helpers.get_table_row_by_column_value(page, "Was?", "Beach-Volleyball")
    expect(row).to_be_visible()

    page.get_by_role("link", name="Kategorien").click()
    page.get_by_role("link", name="Sport").click()
    row = helpers.get_table_row_by_column_value(page, "Was?", "Beach-Volleyball")
    expect(row).to_be_visible()

    page.get_by_role("link", name="Kategorien").click()
    page.get_by_role("link", name="Default").click()
    expect(page.get_by_text("Beach-Volleyball")).not_to_be_visible()


def test_main_list_entry_correct_pages_and_carry(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_entry(page, data.ENTRY_SONNENAUFGANG_WANDERUNG)

    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Sa 04.01.").click()
    expect(page.get_by_text("Sonnenaufgang-Wanderung")).not_to_be_visible()

    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="So 05.01.").click()
    row = helpers.get_table_row_by_column_value(page, "Was?", "Sonnenaufgang-Wanderung")
    expect(row).to_be_visible()
    expect(row).not_to_contain_text("Übertrag vom Vortag")

    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Mo 06.01.").click()
    row = helpers.get_table_row_by_column_value(page, "Was?", "Sonnenaufgang-Wanderung")
    expect(row).to_be_visible()
    expect(row).to_contain_text("Übertrag vom Vortag")

    page.get_by_role("link", name="Kategorien").click()
    page.get_by_role("link", name="Default").click()
    row = helpers.get_table_row_by_column_value(page, "Was?", "Sonnenaufgang-Wanderung")
    expect(row).to_be_visible()

    page.get_by_role("link", name="Orte").click()
    page.get_by_role("link", name="KüAs ohne Ort").click()
    row = helpers.get_table_row_by_column_value(page, "Was?", "Sonnenaufgang-Wanderung")
    expect(row).to_be_visible()


def test_main_list_order(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_entry(page, data.ENTRY_WEST_COAST_SWING)
    actions.add_entry(page, data.ENTRY_AKROBATIK)
    actions.add_entry(page, data.ENTRY_TANZABEND)
    expect(page).to_have_title(re.compile(r"03\.01\."))
    expect(
        page.locator("xpath=//td[1]").or_(page.get_by_title("Kalenderdatum").locator("xpath=ancestor-or-self::td"))
    ).to_contain_text(["Akrobatik", "Tanzabend", "04.01. 00:00", "West Coast Swing"])


def test_main_list_description(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_entry(page, data.ENTRY_LOREM_IPSUM)

    # Check presence of headings
    expect(page.get_by_role("heading")).to_contain_text(
        ["Beschreibungen", "Lorem Ipsum dolor sit amet", "Vulputate qui blandit praesent", "Ullamcorper lobortis"]
    )

    # Check heading levels
    expect(page.get_by_role("heading", name="Beschreibungen")).to_have_js_property("tagName", "H2")
    expect(page.get_by_role("heading", name="Lorem Ipsum dolor sit amet")).to_have_js_property("tagName", "H3")
    expect(page.get_by_role("heading", name="Vulputate qui blandit praesent")).to_have_js_property("tagName", "H4")

    # Check further Markdown formatting
    section = page.get_by_role("heading", name="Lorem Ipsum dolor sit amet").locator("xpath=ancestor-or-self::section")
    expect(section.get_by_role("listitem").first).to_have_text("At vero eos et accusam")
    link = section.get_by_role("link", name="dolore eu feugiat nulla facilisis")
    expect(link).to_be_visible()
    expect(link).to_have_attribute("href", "https://example.com")

    # Check presence of meta data above description
    metadata_paragraph = section.get_by_text("von incognita")
    expect(metadata_paragraph).to_contain_text("04.01. 14:00")
