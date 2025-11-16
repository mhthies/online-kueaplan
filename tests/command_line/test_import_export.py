import re
import subprocess
from pathlib import Path

from playwright.sync_api import Page, expect

from ..ui import actions
from ..ui.data import ANNOUNCEMENT_SPORTPLATZ_NASS, CATEGORY_SPORT, ENTRY_BEACH_VOLLEYBALL, ROOM_SPORTPLAETZE
from . import cli_actions


def test_export_and_reimport(
    page: Page, kueaplan_server_executable_or_skip: Path, tmp_path: Path, reset_database: None
) -> None:
    actions.login(page, 1, "orga")
    actions.add_category(page, CATEGORY_SPORT)
    actions.add_room(page, ROOM_SPORTPLAETZE)
    actions.add_entry(page, ENTRY_BEACH_VOLLEYBALL)
    actions.add_announcement(page, ANNOUNCEMENT_SPORTPLATZ_NASS)

    json_file = tmp_path / "export.json"
    subprocess.run([kueaplan_server_executable_or_skip, "event", "export", "1", str(json_file)], check=True)

    result = subprocess.run(
        [kueaplan_server_executable_or_skip, "event", "import", str(json_file)], check=True, stdout=subprocess.PIPE
    )
    match = re.search(rb"imported successfully with id (\d+)", result.stdout)
    assert match, f"'imported successfully with id \\d+' not found in stdout: {result.stdout!r}"
    new_event_id = int(match.group(1))

    cli_actions.create_passphrase(kueaplan_server_executable_or_skip, str(new_event_id), "orga", "orga")

    actions.login(page, new_event_id, "orga")
    page.get_by_role("navigation").get_by_role("link", name="Orte").click()
    expect(page.get_by_role("heading", name="Orte")).to_be_visible()
    page.get_by_role("link", name="Sportplätze").click()
    # Announcement
    expect(page.get_by_text("nass und rutschig")).to_be_visible()
    # Entry
    expect(page.get_by_text("Beach-Volleyball", exact=True)).to_be_visible()
