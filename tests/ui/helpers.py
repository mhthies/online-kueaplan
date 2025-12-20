from re import Pattern

from playwright.sync_api import Locator, Page


def get_table_row_by_column_value(page: Page, header_text: str | Pattern[str], value: str | Pattern[str]) -> Locator:
    """Create a Playwright Locator, looking for a table row with the `value` in the column labeled with `header_text`

    The function looks up the matching header's index and creates a locator for a table row containing the desired value
    in the nth <td> tag. This approach fails when cells with rowspan or colspan are involved.
    """
    header = page.get_by_role("columnheader", name=header_text).element_handle()
    header_index = header.evaluate(
        "node => { let i = 0; while((node = node.previousElementSibling) != null) i++; return i; }"
    )
    cell_locator = page.locator(f"xpath=//td[{header_index + 1}]").filter(has_text=value)
    return page.get_by_role("row").filter(has=cell_locator)


def get_table_cell_by_header(table_row: Locator, header_text: str | Pattern[str]) -> Locator:
    """Create a Playwright Locator, looking for a table cell in the column labeled with `header_text`

    The function looks up the matching header's index and creates a locator for the nth <td> tag. This approach fails
    when cells with rowspan or colspan are involved.
    """
    header = (
        table_row.locator("xpath=ancestor-or-self::table")
        .get_by_role("columnheader", name=header_text)
        .element_handle()
    )
    header_index = header.evaluate(
        "node => { let i = 0; while((node = node.previousElementSibling) != null) i++; return i; }"
    )
    return table_row.locator(f"xpath=//td[{header_index + 1}]")
