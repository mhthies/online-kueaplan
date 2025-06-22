import re

from playwright.sync_api import Page, expect


def login(page: Page, event_id: int,  passphrase: str):
    page.goto(f"http://localhost:9099/ui/{event_id}")
    expect(page).to_have_title(re.compile("Login"))
    page.get_by_role("textbox", name="Passphrase").fill(passphrase)
    page.get_by_role("button", name="Zum KüA-Plan").click()
    success_alert = page.get_by_role("alert").filter(has_text="Login erfolgreich")
    expect(success_alert).to_be_visible()
    success_alert.get_by_role("button", name="Close").click()
