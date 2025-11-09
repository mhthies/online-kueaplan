import re

from playwright.sync_api import Page, expect

from . import actions


def test_user_cannot_add_entry(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "user")

    expect(page.get_by_role("link", name="Eintrag hinzufügen")).not_to_be_visible()

    page.goto("http://localhost:9099/ui/1/new_entry?date=2025-01-06")
    expect(page).to_have_title(re.compile("Fehler"))
    expect(page.get_by_role("alert").filter(has_text="Zugriff verweigert")).to_be_visible()
    login_link = page.get_by_role("link", name="Zum Login-Formular")
    expect(login_link).to_be_visible()
