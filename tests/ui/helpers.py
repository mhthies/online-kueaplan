import re
from re import Pattern

from playwright.sync_api import Locator, Page


def get_table_row_by_column_value(
    page: Locator | Page, header_text: str | Pattern[str], value: str | Pattern[str]
) -> Locator:
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


def assert_small_font(locator: Locator) -> None:
    """Asserts that the element located by the locator has a smaller font size than normal"""
    font_weight = locator.evaluate("el => window.getComputedStyle(el).getPropertyValue('font-size')")
    font_weight_match = re.match(r"(\d+(\.\d+)?)px", font_weight)
    assert font_weight_match is not None
    font_weight_value = float(font_weight_match.group(1))
    body_font_weight = locator.page.evaluate(
        "() => window.getComputedStyle(document.body).getPropertyValue('font-size')"
    )
    body_font_weight_match = re.match(r"(\d+(\.\d+)?)px", body_font_weight)
    assert body_font_weight_match is not None
    body_font_weight_value = float(body_font_weight_match.group(1))
    assert font_weight_value < body_font_weight_value


def is_line_through(locator: Locator) -> bool:
    return locator.locator("xpath=ancestor-or-self::*").evaluate_all("""
        (elements) => elements.some(
            (e) => (
                getComputedStyle(e)
                .getPropertyValue('text-decoration-line')
                == 'line-through'))""")
