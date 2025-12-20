import datetime
import re

from playwright.sync_api import BrowserContext, Page, expect

from . import actions


def test_entity_not_found(page: Page) -> None:
    # Non-existing event id 123
    page.goto("http://localhost:9099/ui/123")
    expect(page.get_by_role("alert").filter(has_text="existiert nicht")).to_be_visible()
    expect(page.get_by_text("Anton Administrator")).to_be_visible()
    expect(page.get_by_text("anton@example.com")).to_be_visible()
    expect(page.get_by_text("EntityNotFound")).to_be_visible()


def test_invalid_path_data(page: Page) -> None:
    # Invalid event id "abc"
    page.goto("http://localhost:9099/ui/abc")
    expect(page.get_by_role("alert").filter(has_text="Daten sind ungültig")).to_be_visible()
    expect(page.get_by_text("Anton Administrator")).to_be_visible()
    expect(page.get_by_text("anton@example.com")).to_be_visible()
    expect(page.get_by_text('Path deserialize error: can not parse "abc" to a i32')).to_be_visible()


def test_privilege_error_user(page: Page) -> None:
    page.goto("http://localhost:9099/ui/1/list/2025-01-04")
    alert = page.get_by_role("alert")
    expect(alert).to_contain_text("Zugriff verweigert")
    expect(
        page.get_by_text(
            re.compile(
                "Damit der KüA-Plan nur anwesenden Teilnehmer\\*innen auf der Akademie zugänglich ist, wird er durch "
                "eine Passphrase geschützt."
            )
        )
    ).to_be_visible()
    page.get_by_role("link", name="Zum Login-Formular").click()

    page.get_by_role("heading", name="KüA-Plan Login").click()
    expect(
        page.get_by_text(
            "Damit der KüA-Plan nur anwesenden Teilnehmer*innen auf der Akademie zugänglich ist, wird er durch eine "
            "Passphrase geschützt."
        )
    ).to_be_visible()
    page.get_by_role("textbox", name="Passphrase").fill("user")
    page.get_by_role("button", name="Zum KüA-Plan").click()

    # The target URL should be kept throughout the whole process, so we should land on the main_list for 2025-01-04
    expect(page).to_have_title(re.compile(r"04\.01\."))


def test_privilege_error_orga(page: Page) -> None:
    actions.login(page, 1, "user")
    page.goto("http://localhost:9099/ui/1/config")
    alert = page.get_by_role("alert")
    expect(alert).to_contain_text("Zugriff verweigert")
    expect(alert).to_contain_text("Authentifizierung als Orga oder Admin erforderlich")
    page.get_by_role("link", name="Zum Login-Formular").click()

    page.get_by_role("heading", name="Erweiterter Login").click()
    expect(
        page.get_by_text(
            "Bitte gib hier eine passende Passphrase ein, die dich als Orga oder Admin zu authentifiziert."
        )
    ).to_be_visible()
    page.get_by_role("textbox", name="Passphrase").fill("orga")
    page.get_by_role("button", name="Login").click()

    # The target URL should be kept throughout the whole process, so we should land on the config index page
    expect(page).to_have_title(re.compile("Konfiguration"))


def test_session_error(context: BrowserContext) -> None:
    # Set cookie to broken value
    context.add_cookies(
        [
            dict(
                name="kuea-plan-session",
                value="XXXXXXXXXXXXX",
                url="http://localhost:9099/",
                expires=(datetime.datetime.now(datetime.UTC) + datetime.timedelta(days=1)).timestamp(),
            )
        ]
    )
    page = context.new_page()
    page.goto("http://localhost:9099/ui/1/list/2025-01-04")
    alert = page.get_by_role("alert")
    expect(alert).to_contain_text("Zugriff verweigert")
    expect(
        page.get_by_text(re.compile("Deine bereits eingegebenen Passphrasen konnten leider nicht geladen werden"))
    ).to_be_visible()
    expect(page.get_by_text(re.compile("Der Session-Token ist ungültig"))).to_be_visible()
    page.get_by_role("button", name="Login-Daten bereinigen").click()
    expect(page.get_by_role("alert").filter(has_text="Erfolg")).to_be_visible()
    page.get_by_role("link", name="Zum Login-Formular").click()

    page.get_by_role("heading", name="KüA-Plan Login").click()
    page.get_by_role("textbox", name="Passphrase").fill("user")
    page.get_by_role("button", name="Zum KüA-Plan").click()
    expect(page.get_by_role("alert").filter(has_text="Erfolg")).to_be_visible()

    # The target URL should be kept throughout the whole process, so we should land on the main_list for 2025-01-04
    expect(page).to_have_title(re.compile(r"04\.01\."))
