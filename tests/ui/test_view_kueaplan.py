import re
from playwright.sync_api import Page, expect

from . import actions

def test_empty_day_notification(page: Page, reset_database: None):
    actions.login(page, 1, "user")
    expect(page).to_have_title(re.compile(r"06\.01\."))
    expect(page.get_by_role("heading", name="KüA-Plan")).to_be_visible()
    expect(page.get_by_text("keine KüAs geplant")).to_be_visible()

def test_empty_category_notification(page: Page, reset_database: None):
    actions.login(page, 1, "user")

    page.get_by_role("navigation").get_by_role("link", name="Kategorien").click()
    expect(page.get_by_role("heading", name="KüA-Kategorien")).to_be_visible()

    page.get_by_role("link", name="Default").click()
    expect(page.get_by_role("heading", name="KüA-Plan")).to_be_visible()
    expect(page.get_by_text(re.compile(r"keine KüAs in der Kategorie .*? geplant"))).to_be_visible()
