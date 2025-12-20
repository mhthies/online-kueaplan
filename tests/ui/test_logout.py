import re

from playwright.sync_api import Page, expect

from . import actions


def test_logout_from_role(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "user")
    actions.login(page, 1, "admin")
    expect(page.get_by_role("link", name="Konfiguration")).to_be_visible()

    page.get_by_role("link", name="Logout").click()
    # Due to the rowspan cells, our fancy get_table_row_by_column_value() helper does not work here
    table_row = page.get_by_role("row").filter(has=page.get_by_role("cell").filter(has_text="Admin"))
    expect(table_row).to_be_visible()
    table_row.get_by_role("button").and_(table_row.get_by_title("Aus dieser Rolle ausloggen")).click()
    expect(page.get_by_role("alert").filter(has_text="Erfolg")).to_be_visible()

    table_row = page.get_by_role("row").filter(has=page.get_by_role("cell").filter(has_text="Admin"))
    expect(table_row).not_to_be_visible()

    page.get_by_role("link", name="TestEvent").click()
    expect(page).to_have_title(re.compile(r"06\.01\."))
    expect(
        page.get_by_role("navigation", name="Haupt-Navigation").get_by_role("link", name="Konfiguration")
    ).not_to_be_visible()


def test_logout_all(page: Page, reset_database: None) -> None:
    actions.login(page, 1, "user")
    actions.login(page, 1, "admin")
    page.goto("http://localhost:9099/")

    # Link to Login-Status page should be labeled "Login-Status" instead of "Logout", when not in an event context.
    page.get_by_role("link", name="Login-Status").click()
    # Due to the rowspan cells, our fancy get_table_row_by_column_value() helper does not work here
    table_row = page.get_by_role("row").filter(has=page.get_by_role("cell").filter(has_text="Admin"))
    expect(table_row).to_be_visible()

    page.get_by_role("button", name="Logout aus allen").click()
    expect(page.get_by_role("alert").filter(has_text="Erfolg")).to_be_visible()

    table_row = page.get_by_role("row").filter(has=page.get_by_role("cell").filter(has_text="Admin"))
    expect(table_row).not_to_be_visible()
    expect(page.get_by_role("link", name="TestEvent")).not_to_be_visible()

    page.goto("http://localhost:9099/ui/1")
    expect(page).to_have_title(re.compile("Login"))
