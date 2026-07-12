from playwright.sync_api import Browser, Page, expect

from . import actions, data, helpers


def test_create_edit_and_publish_draft(page: Page, browser: Browser, reset_database: None) -> None:
    user_context = browser.new_context()
    user_page = user_context.new_page()
    actions.login(user_page, 1, "user")

    actions.login(page, 1, "orga")
    actions.add_entry(page, data.ENTRY_TANZABEND, as_draft=True)

    # Draft should not be shown for users or orgas in normal plan
    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Fr 03.01.").click()
    expect(page.get_by_role("document")).not_to_contain_text("Tanzabend")
    user_page.get_by_role("button", name="Datum").click()
    user_page.get_by_role("link", name="Fr 03.01.").click()
    expect(user_page.get_by_role("document")).not_to_contain_text("Tanzabend")

    page.get_by_role("link", name="Versteckte").click()
    page.get_by_role("link", name="Entwürfe").click()
    row = helpers.get_table_row_by_column_value(page, "Was?", "Tanzabend")
    row.get_by_role("link", name="Eintrag bearbeiten").click()
    page.get_by_role("textbox", name="von wem? / Ansprechpersonen").fill("Anna")
    expect(page.get_by_role("radio", name="Als Entwurf belassen")).to_be_checked()
    page.get_by_role("button", name="Speichern").click()
    expect(helpers.get_table_cell_by_header(row, "Von wem?")).to_contain_text("Anna")

    # public visibility should not have changed
    user_page.reload()
    expect(user_page.get_by_role("document")).not_to_contain_text("Tanzabend")

    row.get_by_role("link", name="Eintrag bearbeiten").click()
    page.locator('label:has-text("Veröffentlichen")').click()
    page.get_by_role("button", name="Speichern").click()

    user_page.reload()
    expect(user_page.get_by_role("document")).to_contain_text("Tanzabend")


# TODO test delete entry


def test_retract_and_republish_entry(page: Page, browser: Browser, reset_database: None) -> None:
    user_context = browser.new_context()
    user_page = user_context.new_page()
    actions.login(user_page, 1, "user")

    actions.login(page, 1, "orga")
    actions.add_entry(page, data.ENTRY_TANZABEND)

    helpers.get_table_row_by_column_value(page, "Was?", "Tanzabend").get_by_role("link", name="bearbeiten").click()
    page.get_by_role("link", name="Löschen").click()
    page.get_by_role("button", name="Verstecken").click()
    actions.check_success_toast(page)

    # Retracted entry should not be shown for users or orgas in normal plan
    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Fr 03.01.").click()
    expect(page.get_by_role("document")).not_to_contain_text("Tanzabend")
    user_page.get_by_role("button", name="Datum").click()
    user_page.get_by_role("link", name="Fr 03.01.").click()
    expect(user_page.get_by_role("document")).not_to_contain_text("Tanzabend")

    # Edit without changing state, using "retracted" page
    page.get_by_role("link", name="Versteckte").click()
    row = helpers.get_table_row_by_column_value(page, "Was?", "Tanzabend")
    row.get_by_role("link", name="Eintrag bearbeiten").click()
    page.get_by_role("textbox", name="von wem? / Ansprechpersonen").fill("Anna")
    expect(page.get_by_role("radio", name="Versteckt lassen")).to_be_checked()
    page.get_by_role("button", name="Speichern").click()
    expect(helpers.get_table_cell_by_header(row, "Von wem?")).to_contain_text("Anna")

    # public visibility should not have changed
    user_page.reload()
    expect(user_page.get_by_role("document")).not_to_contain_text("Tanzabend")

    row.get_by_role("link", name="Eintrag bearbeiten").click()
    page.locator('label:has-text("Veröffentlichen")').click()
    page.get_by_role("button", name="Speichern").click()

    user_page.reload()
    expect(user_page.get_by_role("document")).to_contain_text("Tanzabend")


# TODO edit submitted entry without deciding, then publish it later.

# TODO test reject submitted entry, edit it and then publish it
