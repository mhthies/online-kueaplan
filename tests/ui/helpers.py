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
    font_size = locator.evaluate("el => window.getComputedStyle(el).getPropertyValue('font-size')")
    font_size_match = re.match(r"(\d+(\.\d+)?)px", font_size)
    assert font_size_match is not None
    font_size_value = float(font_size_match.group(1))
    body_font_size = locator.page.evaluate("() => window.getComputedStyle(document.body).getPropertyValue('font-size')")
    body_font_size_match = re.match(r"(\d+(\.\d+)?)px", body_font_size)
    assert body_font_size_match is not None
    body_font_size_value = float(body_font_size_match.group(1))
    assert font_size_value < body_font_size_value


def is_line_through(locator: Locator) -> bool:
    return locator.locator("xpath=ancestor-or-self::*").evaluate_all("""
        (elements) => elements.some(
            (e) => (
                getComputedStyle(e)
                .getPropertyValue('text-decoration-line')
                == 'line-through'))""")


def is_text_bold(locator: Locator) -> bool:
    css_font_weight = locator.evaluate("el => window.getComputedStyle(el).getPropertyValue('font-weight')")
    return int(css_font_weight) > 400


def is_text_colored(locator: Locator) -> bool:
    css_color = locator.evaluate("el => window.getComputedStyle(el).getPropertyValue('color')")
    match = re.search(r"\((\d+), (\d+), (\d+)(?:, ([\d.]+))?\)", css_color)
    assert match
    r, g, b = int(match.group(1)), int(match.group(2)), int(match.group(3))
    return abs(min(r, g, b) - max(r, g, b)) > 20
