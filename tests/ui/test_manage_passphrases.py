import re

from playwright.sync_api import Browser, Page, expect

from tests.ui import actions, helpers


def test_create_new_user_passphrase(browser: Browser, reset_database: None) -> None:
    user_context = browser.new_context()
    user_page = user_context.new_page()
    user_page.goto("http://localhost:9099/ui/1")
    expect(user_page).to_have_title(re.compile("Login"))
    user_page.get_by_role("textbox", name="Passphrase").fill("test-passphrase")
    user_page.get_by_role("button", name="Zum KüA-Plan").click()
    error_alert = user_page.get_by_role("alert").filter(has_text="Ungültige Passphrase.")
    expect(error_alert).to_be_visible()

    admin_context = browser.new_context()
    admin_page = admin_context.new_page()
    actions.login(admin_page, 1, "admin")
    admin_page.get_by_role("link", name="Konfiguration").click()
    admin_page.get_by_role("link", name="Passphrasen").click()
    admin_page.get_by_role("link", name="Passphrase hinzufügen").click()
    admin_page.get_by_role("textbox", name="Passphrase").fill("test-passphrase")
    admin_page.get_by_role("combobox", name="Rolle/Berechtigung").select_option("User")
    admin_page.get_by_role("button", name="Erstellen").click()

    actions.login(user_page, 1, "test-passphrase")
    expect(user_page.get_by_role("link", name="Eintrag hinzufügen")).not_to_be_visible()


def test_create_sharable_link_passphrase_for_admin(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "admin")

    page.get_by_role("link", name="Links für Kalender").click()
    expect(page.get_by_text("Konnte keinen Authentifizierungs-Token für Link-Freigabe erzeugen.")).to_be_visible()

    page.get_by_role("link", name="Konfiguration").click()
    page.get_by_role("link", name="Passphrasen").click()

    table_row = helpers.get_table_row_by_column_value(page, "Rolle", "Admin")
    expect(table_row).to_be_visible()
    actions_cell = helpers.get_table_cell_by_header(table_row, "Aktionen")
    actions_cell.get_by_role("link").and_(actions_cell.get_by_title("Abruf per Link hinzufügen")).click()
    page.get_by_role("button", name="Erstellen").click()
    expect(page.get_by_role("alert").filter(has_text="Erfolg")).to_be_visible()

    page.get_by_role("link", name="Links für Kalender").click()
    expect(page.get_by_role("textbox", name="iCal-Link")).to_be_visible()


def test_delete_user_passphrase(browser: Browser, reset_database: None) -> None:
    user_context = browser.new_context()
    user_page = user_context.new_page()
    actions.login(user_page, 1, "user")

    admin_context = browser.new_context()
    admin_page = admin_context.new_page()
    actions.login(admin_page, 1, "admin")
    admin_page.get_by_role("link", name="Konfiguration").click()
    admin_page.get_by_role("link", name="Passphrasen").click()

    table_row = helpers.get_table_row_by_column_value(admin_page, "Rolle", "User")
    expect(table_row).to_be_visible()
    actions_cell = helpers.get_table_cell_by_header(table_row, "Aktionen")
    actions_cell.get_by_role("link", name="Passphrase löschen").click()
    admin_page.get_by_role("button", name="Löschen").click()
    expect(
        admin_page.get_by_role("alert").filter(has_text="Die Passphrase/Ableitbare Rolle wurde gelöscht.")
    ).to_be_visible()

    user_page.reload()
    expect(user_page.get_by_text("Zugriff verweigert")).to_be_visible()

    user_context2 = browser.new_context()
    user_page2 = user_context2.new_page()
    user_page2.goto("http://localhost:9099/ui/1")
    expect(user_page2).to_have_title(re.compile("Login"))
    user_page2.get_by_role("textbox", name="Passphrase").fill("user")
    user_page2.get_by_role("button", name="Zum KüA-Plan").click()
    error_alert = user_page2.get_by_role("alert").filter(has_text="Ungültige Passphrase.")
    expect(error_alert).to_be_visible()


def test_invalidate_user_passphrase(browser: Browser, reset_database: None) -> None:
    user_context = browser.new_context()
    user_page = user_context.new_page()
    actions.login(user_page, 1, "user")

    admin_context = browser.new_context()
    admin_page = admin_context.new_page()
    actions.login(admin_page, 1, "admin")
    admin_page.get_by_role("link", name="Konfiguration").click()
    admin_page.get_by_role("link", name="Passphrasen").click()

    table_row = helpers.get_table_row_by_column_value(admin_page, "Rolle", "User")
    expect(table_row).to_be_visible()
    actions_cell = helpers.get_table_cell_by_header(table_row, "Aktionen")
    actions_cell.get_by_role("link", name="Passphrase löschen").click()
    admin_page.get_by_role("button", name="Ungültig machen").click()
    expect(
        admin_page.get_by_role("alert").filter(has_text="Die Passphrase/Ableitbare Rolle wurde ungültig gemacht.")
    ).to_be_visible()

    user_page.reload()
    expect(user_page.get_by_text("Zugriff verweigert")).to_be_visible()
    expect(user_page.get_by_text("nicht mehr gültig")).to_be_visible()

    user_context2 = browser.new_context()
    user_page2 = user_context2.new_page()
    user_page2.goto("http://localhost:9099/ui/1")
    expect(user_page2).to_have_title(re.compile("Login"))
    user_page2.get_by_role("textbox", name="Passphrase").fill("user")
    user_page2.get_by_role("button", name="Zum KüA-Plan").click()
    error_alert = user_page2.get_by_role("alert")
    expect(error_alert).to_be_visible()
    expect(error_alert).to_have_text(re.compile(r".*nicht mehr \(oder noch nicht\) gültig\..*"))
