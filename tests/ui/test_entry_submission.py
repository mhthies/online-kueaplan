import re

from playwright.sync_api import Browser, expect

from . import actions, helpers


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
    row.get_by_role("link", name="Eintrag bearbeiten").click()
    orga_page.get_by_role("radio", name="Veröffentlichen").check(force=True)
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


# TODO test all fields of submit entry form

# TODO test preview in submit entry form

# TODO test tab navigation + next/prev buttons + browser back/forwar in entry submission form


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

    orga_page.reload()
    review_area_button = orga_page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Prüfen")
    expect(review_area_button).to_contain_text("1")
    review_area_button.click()
    expect(orga_page).to_have_title(re.compile("Zu prüfende Einträge"))
    row = helpers.get_table_row_by_column_value(orga_page, "Was?", "Drachenfliegen leicht gemacht")
    row.get_by_role("link", name="Eintrag bearbeiten").click()
    orga_page.get_by_role("radio", name="Bestätigen").check(force=True)
    orga_page.get_by_role("button", name="Speichern").click()
    actions.check_success_toast(orga_page)
    expect(review_area_button).not_to_contain_text("1")
    review_area_button.click()
    expect(orga_page).to_have_title(re.compile("Zu prüfende Einträge"))
    expect(orga_page.get_by_role("document")).not_to_contain_text("Drachenfliegen leicht gemacht")


# TODO test that "official" categories are not selectable

# TODO test going to "Eintrag einreichen" page or manually POSTing entry submission does not work when disabled
