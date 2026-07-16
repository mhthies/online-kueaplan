import re

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

    # Draft should not be counted into entry count of public category/room lists
    user_page2 = user_context.new_page()
    user_page2.goto(user_page.url)
    user_page2.get_by_role("link", name="Kategorien").click()
    expect(user_page2.get_by_role("link", name="Default")).to_contain_text("0 Einträge")

    # Edit draft without publishing
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

    # Publish entry
    row.get_by_role("link", name="Eintrag bearbeiten").click()
    page.locator('label:has-text("Veröffentlichen")').click()
    page.get_by_role("button", name="Speichern").click()

    # entry should be visible to user
    user_page.reload()
    expect(user_page.get_by_role("document")).to_contain_text("Tanzabend")
    # ... and counted into category's entry count
    user_page2.reload()
    expect(user_page2.get_by_role("link", name="Default")).to_contain_text("1 Eintrag")


def test_delete_entry(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_entry(page, data.ENTRY_TANZABEND)

    helpers.get_table_row_by_column_value(page, "Was?", "Tanzabend").get_by_role("link", name="bearbeiten").click()
    page.get_by_role("link", name="Entfernen").click()
    page.get_by_role("button", name="Löschen").click()
    actions.check_success_toast(page)

    # Deleted entry should not be shown in normal plan, nor in "retracted" page
    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Fr 03.01.").click()
    expect(page.get_by_role("document")).not_to_contain_text("Tanzabend")
    page.get_by_role("link", name="Versteckte").click()
    expect(page.get_by_role("document")).not_to_contain_text("Tanzabend")


def test_retract_and_delete_entry(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "orga")
    actions.add_entry(page, data.ENTRY_TANZABEND)

    # Retract entry
    helpers.get_table_row_by_column_value(page, "Was?", "Tanzabend").get_by_role("link", name="bearbeiten").click()
    page.get_by_role("link", name="Entfernen").click()
    page.get_by_role("button", name="Verstecken").click()
    actions.check_success_toast(page)

    # Retracted entry should not be shown for users or orgas in normal plan
    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Fr 03.01.").click()
    expect(page.get_by_role("document")).not_to_contain_text("Tanzabend")

    # Delete entry
    page.get_by_role("link", name="Versteckte").click()
    helpers.get_table_row_by_column_value(page, "Was?", "Tanzabend").get_by_role("link", name="bearbeiten").click()
    page.get_by_role("link", name="Entfernen").click()
    page.get_by_role("button", name="Löschen").click()
    actions.check_success_toast(page)

    # Deleted entry should not be shown in normal plan, nor in "retracted" page
    page.get_by_role("button", name="Datum").click()
    page.get_by_role("link", name="Fr 03.01.").click()
    expect(page.get_by_role("document")).not_to_contain_text("Tanzabend")
    page.get_by_role("link", name="Versteckte").click()
    expect(page.get_by_role("document")).not_to_contain_text("Tanzabend")


def test_retract_and_republish_entry(page: Page, browser: Browser, reset_database: None) -> None:
    user_context = browser.new_context()
    user_page = user_context.new_page()
    actions.login(user_page, 1, "user")

    actions.login(page, 1, "orga")
    actions.add_entry(page, data.ENTRY_TANZABEND)

    helpers.get_table_row_by_column_value(page, "Was?", "Tanzabend").get_by_role("link", name="bearbeiten").click()
    page.get_by_role("link", name="Entfernen").click()
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


def test_reject_and_republish_submitted_entry(page: Page, browser: Browser, reset_database: None) -> None:
    orga_page = page
    actions.login(orga_page, 1, "admin")
    actions.enable_entry_submission(orga_page, True)

    user_context = browser.new_context()
    user_page = user_context.new_page()
    actions.login(user_page, 1, "user")

    # Submit entry
    user_page.get_by_role("link", name="Eintrag einreichen").click()
    user_page.get_by_role("textbox", name="Titel der KüA").fill("Drachenfliegen leicht gemacht")
    user_page.get_by_role("button", name="Weiter").click()
    user_page.get_by_role("textbox", name="Beginn").fill("13:00")
    user_page.get_by_role("tab", name="Vorschau").click()
    expect(user_page.get_by_role("checkbox", name="Direkt veröffentlichen")).to_be_checked()
    user_page.get_by_role("checkbox", name="Ich habe die Vorschau geprüft").check()
    user_page.get_by_role("button", name="Veröffentlichen").click()
    actions.check_success_toast(user_page)

    # User should already see the entry in the public list
    expect(user_page).to_have_title(re.compile(r"06\.01\."))
    expect(user_page.get_by_role("document")).to_contain_text("Drachenfliegen leicht gemacht")

    # Reject entry
    orga_page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Prüfen").click()
    helpers.get_table_row_by_column_value(orga_page, "Was?", "Drachenfliegen leicht gemacht").get_by_role(
        "link", name="bearbeiten"
    ).click()
    orga_page.locator('label:has-text("Ablehnen")').click()
    orga_page.get_by_role("button", name="Speichern").click()
    actions.check_success_toast(orga_page)

    # User should not see entry anymore
    user_page.reload()
    expect(user_page.get_by_role("document")).not_to_contain_text("Drachenfliegen leicht gemacht")
    # and it should not be counted into entry count of public category/room lists anymore
    user_page2 = user_context.new_page()
    user_page2.goto(user_page.url)
    user_page2.get_by_role("link", name="Kategorien").click()
    expect(user_page2.get_by_role("link", name="Default")).to_contain_text("0 Einträge")

    # Entry should not be listed as "to review" anymore, but it should be listed as "rejected"
    orga_page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Prüfen").click()
    expect(orga_page).to_have_title(re.compile("Zu prüfende Einträge"))
    expect(orga_page.get_by_role("document")).not_to_contain_text("Drachenfliegen leicht gemacht")
    rejected_list_link = orga_page.get_by_role("link", name="Abgelehnt")
    expect(rejected_list_link).to_contain_text("1")
    rejected_list_link.click()
    expect(helpers.get_table_row_by_column_value(orga_page, "Was?", "Drachenfliegen leicht gemacht")).to_be_visible()

    # Edit entry (new responsible person) without changing state
    helpers.get_table_row_by_column_value(orga_page, "Was?", "Drachenfliegen leicht gemacht").get_by_role(
        "link", name="bearbeiten"
    ).click()
    orga_page.get_by_role("textbox", name="von wem? / Ansprechpersonen").fill("Anna")
    expect(orga_page.get_by_role("radio", name="Versteckt lassen")).to_be_checked()
    orga_page.get_by_role("button", name="Speichern").click()
    actions.check_success_toast(orga_page)
    expect(orga_page).to_have_title(re.compile("Abgelehnte Einreichungen"))

    # User should still not see entry anymore
    user_page.reload()
    expect(user_page.get_by_role("document")).not_to_contain_text("Drachenfliegen leicht gemacht")

    # Republish entry
    helpers.get_table_row_by_column_value(orga_page, "Was?", "Drachenfliegen leicht gemacht").get_by_role(
        "link", name="bearbeiten"
    ).click()
    expect(page.get_by_role("document")).to_contain_text("Bei Prüfung abgelehnt")
    page.locator('label:has-text("Veröffentlichen")').click()
    orga_page.get_by_role("button", name="Speichern").click()
    actions.check_success_toast(orga_page)

    # User should see entry again, with new responsible person
    user_page.reload()
    row = helpers.get_table_row_by_column_value(user_page, "Was?", "Drachenfliegen leicht gemacht")
    expect(row).to_be_visible()
    expect(row.get_by_role("cell").nth(3)).to_contain_text("Anna")
    # and it should be counted into the category's entry count again
    user_page2.reload()
    expect(user_page2.get_by_role("link", name="Default")).to_contain_text("1 Eintrag")


def test_edit_submitted_entry_and_publish_later(page: Page, browser: Browser, reset_database: None) -> None:
    orga_page = page
    actions.login(orga_page, 1, "admin")
    actions.enable_entry_submission(orga_page, False)

    user_context = browser.new_context()
    user_page = user_context.new_page()
    actions.login(user_page, 1, "user")

    # Submit entry
    user_page.get_by_role("link", name="Eintrag einreichen").click()
    user_page.get_by_role("textbox", name="Titel der KüA").fill("Drachenfliegen leicht gemacht")
    user_page.get_by_role("button", name="Weiter").click()
    user_page.get_by_role("textbox", name="Beginn").fill("13:00")
    user_page.get_by_role("textbox", name="Dauer").fill("1,5")
    user_page.get_by_role("tab", name="Vorschau").click()
    user_page.get_by_role("checkbox", name="Ich habe die Vorschau geprüft").check()
    user_page.get_by_role("button", name="Einreichen").click()
    actions.check_success_toast(user_page)

    # Entry should not be visible in the plan yet
    expect(user_page).to_have_title(re.compile(r"06\.01\."))
    expect(user_page.get_by_role("document")).not_to_contain_text("Drachenfliegen leicht gemacht")

    # Edit entry without publishing
    orga_page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Prüfen").click()
    helpers.get_table_row_by_column_value(orga_page, "Was?", "Drachenfliegen leicht gemacht").get_by_role(
        "link", name="Eintrag bearbeiten"
    ).click()
    orga_page.get_by_role("textbox", name="von wem? / Ansprechpersonen").fill("Anna")
    expect(orga_page.get_by_role("radio", name="Ungeprüft")).to_be_checked()
    orga_page.get_by_role("button", name="Speichern").click()
    actions.check_success_toast(orga_page)

    # Entry should still not be visible in the plan yet
    user_page.reload()
    expect(user_page.get_by_role("document")).not_to_contain_text("Drachenfliegen leicht gemacht")

    # Review area button in the navigation bar should still show one entry to review
    review_area_button = orga_page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Prüfen")
    expect(review_area_button).to_contain_text("1")

    # Publish entry
    helpers.get_table_row_by_column_value(orga_page, "Was?", "Drachenfliegen leicht gemacht").get_by_role(
        "link", name="Eintrag bearbeiten"
    ).click()
    orga_page.locator('label:has-text("Veröffentlichen")').click()
    orga_page.get_by_role("button", name="Speichern").click()
    actions.check_success_toast(orga_page)

    expect(review_area_button).not_to_contain_text("1")

    # User should see entry now, with new responsible person
    user_page.reload()
    row = helpers.get_table_row_by_column_value(user_page, "Was?", "Drachenfliegen leicht gemacht")
    expect(row).to_be_visible()
    expect(row.get_by_role("cell").nth(3)).to_contain_text("Anna")
