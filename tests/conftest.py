import json
import os
from pathlib import Path
import shutil
import subprocess
import time

import dotenv
import pytest


def pytest_addoption(parser: pytest.Parser):
    parser.addoption(
        "--start-app",
        action="store_true",
        help="Start the kuaeplan application server before running the tests",
    )


def _cargo_build_and_get_executable_path(working_directory: Path) -> str:
    result = subprocess.run([shutil.which("cargo"), "build", "--message-format=json"], check=True,
                            cwd=working_directory, stdout=subprocess.PIPE)
    for line in result.stdout.decode(errors='replace').splitlines():
        data = json.loads(line)
        if data.get("reason") == "compiler-artifact" and data.get("executable"):
            return data["executable"]


@pytest.yield_fixture(scope="session", autouse=True)
def start_kueaplan_server(request: pytest.FixtureRequest):
    if not request.config.getoption("--start-app"):
        return
    executable_path = _cargo_build_and_get_executable_path(Path(__file__).parent.parent / "server")
    cmd = [executable_path, "serve"]
    process = subprocess.Popen(cmd)
    time.sleep(2)
    returncode = process.poll()
    if returncode is not None:
        raise subprocess.CalledProcessError(returncode, cmd)
    yield
    process.terminate()
    process.wait(timeout=5)
    process.kill()


@pytest.fixture(scope="session")
def load_dotenv():
    dotenv.load_dotenv()


@pytest.fixture(scope="function")
def reset_database(request: pytest.FixtureRequest, load_dotenv: None):
    database_dump = request.node.get_closest_marker("database_dump")
    if database_dump is None:
        database_dump = "empty.sql"
    database_file = Path(__file__).parent / "database_dumps" / database_dump
    assert isinstance(database_dump, str)
    result = subprocess.run([shutil.which("psql"), "-f", str(database_file), os.environ["DATABASE_URL"]],
                            capture_output=True)
    if result.returncode != 0:
        print(result.stdout)
        raise RuntimeError(f"Could not restore database dump. psql exited with return code {result.returncode}")
