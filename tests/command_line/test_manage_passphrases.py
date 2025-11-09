import re
import subprocess
from pathlib import Path

from playwright.sync_api import Page, expect

from ..ui import actions
from . import util


def test_list_existing_passphrases(kueaplan_server_executable_or_skip: Path, reset_database: None) -> None:
    result = subprocess.run(
        [kueaplan_server_executable_or_skip, "passphrase", "list", "1"], check=True, stdout=subprocess.PIPE
    )
    output = result.stdout.decode()
    assert re.search(r"TestEvent", output)
    assert re.search(r"\|\s*3\s*Admin\s*\*\*\*\*n", output)
    assert re.search(r"\|\s*4.*Link\s*1", output)


def test_list_existing_passphrases_by_event_slug(
    kueaplan_server_executable_or_skip: Path, reset_database: None
) -> None:
    result = subprocess.run(
        [kueaplan_server_executable_or_skip, "passphrase", "list", "test"], check=True, stdout=subprocess.PIPE
    )
    output = result.stdout.decode()
    assert re.search(r"TestEvent", output)
    assert re.search(r"\|\s*3\s*Admin\s*\*\*\*\*n", output)


def test_create_passphrase(page: Page, kueaplan_server_executable_or_skip: Path, reset_database: None) -> None:
    cmd = [kueaplan_server_executable_or_skip, "passphrase", "create", "test"]
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stdin=subprocess.PIPE)
    try:
        util.wait_for_prompt_and_type(process, "access role", "admin")
        util.wait_for_prompt_and_type(process, "passphrase", "very-secret-passphrase")
        util.wait_for_prompt_and_type(process, "derivable passphrase for link-sharing", "y")
        process.wait(1)
        final_output = process.stdout.read()
        assert b"Success" in final_output
        if process.returncode:
            raise subprocess.CalledProcessError(process.returncode, cmd)
    finally:
        process.terminate()
        process.wait(1)
        process.kill()

    actions.login(page, 1, "very-secret-passphrase")
    page.get_by_role("link", name="Konfiguration").click()
    # Check that we have admin privileges
    expect(page.get_by_role("link", name="Passphrasen")).to_be_visible()
    # Check that derivable passphrase for link-sharing is present
    page.get_by_role("link", name="Links für Kalender").click()
    expect(page.get_by_role("textbox", name="iCal-Link")).to_be_visible()


def test_delete_passphrase(page: Page, kueaplan_server_executable_or_skip: Path, reset_database: None) -> None:
    cmd = [kueaplan_server_executable_or_skip, "passphrase", "delete", "test", "1"]
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stdin=subprocess.PIPE)
    try:
        output = util.wait_for_interactive_prompt(process.stdout)
        assert b"'***r'" in output
        assert b"TestEvent" in output
        assert b"want to delete" in output
        process.stdin.write(b"y\n")
        process.stdin.flush()

        process.wait(1)
        if process.returncode:
            raise subprocess.CalledProcessError(process.returncode, cmd)
    finally:
        process.terminate()
        process.wait(1)
        process.kill()

    page.goto("http://localhost:9099/ui/1")
    expect(page).to_have_title(re.compile("Login"))
    page.get_by_role("textbox", name="Passphrase").fill("user")
    page.get_by_role("button", name="Zum KüA-Plan").click()
    alert = page.get_by_role("alert").filter(has_text="Fehler")
    expect(alert).to_be_visible()
