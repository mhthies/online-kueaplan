from re import Pattern

from playwright.sync_api import Locator, Page


def get_table_row_by_column_value(page: Page, header_text: str | Pattern[str], value: str | Pattern[str]) -> Locator:
    header = page.get_by_role("columnheader", name=header_text).element_handle()
    header_index = header.evaluate(
        "node => { let i = 0; while((node = node.previousElementSibling) != null) i++; return i; }")
    cell_locator = page.locator(f"xpath=//td[{header_index+1}]").filter(has_text=value)
    return page.get_by_role("row").filter(has=cell_locator)


def get_table_cell_by_header(table_row: Locator, header_text: str | Pattern[str]) -> Locator:
    header = (table_row
             .locator("xpath=ancestor-or-self::table")
             .get_by_role("columnheader", name=header_text)
             .element_handle())
    header_index = header.evaluate(
        "node => { let i = 0; while((node = node.previousElementSibling) != null) i++; return i; }")
    return table_row.locator(f"xpath=//td[{header_index+1}]")
