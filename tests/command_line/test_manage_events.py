import datetime
import re
import subprocess
import time
from pathlib import Path
from typing import Optional

from playwright.sync_api import Page, expect

from . import util
from ..ui import actions


def test_list_existing_event(kueaplan_server_executable_or_skip: Path, reset_database: None) -> None:
    result = subprocess.run([kueaplan_server_executable_or_skip, "event", "list"], check=True, stdout=subprocess.PIPE)
    output = result.stdout.decode()
    assert re.search(r"1\s*test\s*TestEvent\s*2025-01-01", output)


def test_create_event(page: Page, kueaplan_server_executable_or_skip: Path, reset_database: None) -> None:
    cmd = [kueaplan_server_executable_or_skip, "event", "create"]
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stdin=subprocess.PIPE)
    try:
        util.wait_for_prompt_and_type(process, "event title", "Pfingsten25")
        util.wait_for_prompt_and_type(process, "event slug", "pa25")
        util.wait_for_prompt_and_type(process, "begin", "2025-06-06")
        util.wait_for_prompt_and_type(process, "end", "2025-06-09")

        output = util.wait_for_interactive_prompt(process.stdout)
        match = re.search(rb"created with id (\d+)", output)
        assert match, f"'created with id \d+' not found in output '{output}'"
        event_id = int(match.group(1))
        assert "admin passphrase".encode() in output

        process.stdin.write(f"y\n".encode())
        process.stdin.flush()

        util.wait_for_prompt_and_type(process, "admin passphrase", "very-secret-passphrase")

        process.wait(1)
        if process.returncode:
            raise subprocess.CalledProcessError(process.returncode, cmd)
    finally:
        process.terminate()
        process.wait(1)
        process.kill()

    actions.login(page, event_id, "very-secret-passphrase")
    # After creating an event, we should be able to create an entry with the default category
    actions.add_entry(page, actions.Entry("Test-Eintrag", datetime.date(2025, 6, 7), datetime.time(15, 0),
                                          datetime.timedelta(minutes=90)))


def test_create_event_abort(kueaplan_server_executable_or_skip: Path) -> None:
    cmd = [kueaplan_server_executable_or_skip, "event", "create"]
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stdin=subprocess.PIPE)
    try:
        output = util.wait_for_interactive_prompt(process.stdout)
        assert "event title".encode() in output
        process.terminate()
        process.wait(1)
        assert process.returncode != 0
    finally:
        process.kill()


def test_create_event_retry(kueaplan_server_executable_or_skip: Path) -> None:
    cmd = [kueaplan_server_executable_or_skip, "event", "create"]
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stdin=subprocess.PIPE)
    try:
        util.wait_for_prompt_and_type(process, "event title", "Pfingsten25")
        util.wait_for_prompt_and_type(process, "event slug", "pa25")
        util.wait_for_prompt_and_type(process, "begin", "2025-06------")

        output = util.wait_for_interactive_prompt(process.stdout)
        assert "Error".encode() in output
        assert "invalid characters".encode() in output
        assert "begin".encode() in output
        process.stdin.write(f"2025-06-06\n".encode())
        process.stdin.flush()

        util.wait_for_prompt_and_type(process, "end", "2025-06-09")
        process.terminate()
        process.wait(1)
    finally:
        process.kill()


def test_delete_event(page: Page, kueaplan_server_executable_or_skip: Path, reset_database: None) -> None:
    # First create a new event to check that only one of them is deleted
    cmd = [kueaplan_server_executable_or_skip, "event", "create"]
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stdin=subprocess.PIPE)
    try:
        util.wait_for_prompt_and_type(process, "event title", "Pfingsten25")
        util.wait_for_prompt_and_type(process, "event slug", "pa25")
        util.wait_for_prompt_and_type(process, "begin", "2025-06-06")
        util.wait_for_prompt_and_type(process, "end", "2025-06-09")
        util.wait_for_prompt_and_type(process, "admin passphrase", "n")
        process.wait(1)
    finally:
        process.kill()

    # Now, delete the `test` event
    cmd = [kueaplan_server_executable_or_skip, "event", "delete", "test"]
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stdin=subprocess.PIPE)
    try:
        util.wait_for_prompt_and_type(process, "enter the event's title", "TestEvent")
        process.wait(1)
    finally:
        process.kill()

    page.goto(f"http://localhost:9099/test")
    expect(page.get_by_text("Not found")).to_be_visible()
    page.goto(f"http://localhost:9099/pa25")
    expect(page.get_by_text("Pfingsten25")).to_be_visible()
