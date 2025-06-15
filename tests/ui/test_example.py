import re
from playwright.sync_api import Page, expect

def test_login_as_user(page: Page, reset_database: None):
    page.goto("http://localhost:9099/ui/1")
    expect(page).to_have_title(re.compile("Login"))

    # Login
    page.get_by_role("textbox", name="Passphrase").fill("user")
    page.get_by_role("button", name="Zum KüA-Plan").click()

    expect(page).to_have_title(re.compile(r"06\.01\."))
    expect(page.get_by_role("heading", name=re.compile("KüA-Plan"))).to_be_visible()
