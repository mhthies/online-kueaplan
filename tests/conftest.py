import json
import logging
import os
import secrets
import shutil
import subprocess
import sys
import time
import types
from pathlib import Path
from typing import Generator, Optional

import dotenv
import pytest


def pytest_addoption(parser: pytest.Parser) -> None:
    parser.addoption(
        "--start-app",
        action="store_true",
        help="Start the kuaeplan application server before running the tests",
    )


def _cargo_build_and_get_executable_path(working_directory: Path) -> Optional[Path]:
    cargo_path = shutil.which("cargo")
    assert cargo_path is not None
    result = subprocess.run(
        [cargo_path, "build", "--message-format=json"],
        check=True,
        cwd=working_directory,
        stdout=subprocess.PIPE,
    )
    for line in result.stdout.decode(errors="replace").splitlines():
        data = json.loads(line)
        if data.get("reason") == "compiler-artifact" and data.get("executable"):
            return Path(data["executable"])
    logging.warning("Could not find kueaplan_server executable: compiler-artifact is missing in cargo's JSON output")
    return None


@pytest.fixture(scope="session")
def kueaplan_server_executable() -> Optional[Path]:
    if shutil.which("cargo"):
        return _cargo_build_and_get_executable_path(Path(__file__).parent.parent / "server")
    else:
        # best guess: Is already built and located at ../../target/debug/kueaplan_server
        path = (Path(__file__).parent.parent / "target" / "debug" / "kueaplan_server").resolve()
        if not path.is_file():
            logging.warning("Could not find pre-compiled kueaplan_server at target/debug/kueaplan_server")
            return None
        return path


@pytest.fixture(scope="session")
def kueaplan_server_executable_or_skip(kueaplan_server_executable: Optional[Path]) -> Path:
    if kueaplan_server_executable is None:
        pytest.skip("kueaplan_server executable could not be found")
    return kueaplan_server_executable


@pytest.fixture(scope="session", autouse=True)
def start_kueaplan_server(
    request: pytest.FixtureRequest, load_dotenv: None, kueaplan_server_executable: Optional[Path]
) -> Generator[None, None, None]:
    if not request.config.getoption("--start-app"):
        yield
        return
    if kueaplan_server_executable is None:
        raise RuntimeError(
            "Could not find kueaplan_server executable. Run without --start-app and execute the server manually."
        )

    _restore_database_dump(os.environ["DATABASE_URL"], Path(__file__).parent / "database_dumps" / "minimal.sql")

    cmd = [str(kueaplan_server_executable), "serve"]
    env = dict(os.environ)
    env["LISTEN_PORT"] = "9099"
    env["LISTEN_ADDRESS"] = "127.0.0.1"
    env["ADMIN_NAME"] = "Anton Administrator"
    env["ADMIN_EMAIL"] = "anton@example.com"
    env["SECRET"] = secrets.token_urlsafe(20)
    process = subprocess.Popen(cmd, env=env)
    time.sleep(2)
    returncode = process.poll()
    if returncode is not None:
        raise subprocess.CalledProcessError(returncode, cmd)
    yield
    process.terminate()
    process.wait(timeout=5)
    process.kill()


@pytest.fixture(scope="session")
def load_dotenv() -> None:
    dotenv.load_dotenv()


@pytest.fixture(scope="function")
def reset_database(request: pytest.FixtureRequest, load_dotenv: None) -> None:
    database_dump = request.node.get_closest_marker("database_dump")
    if database_dump is None:
        database_dump = "minimal.sql"
    database_file = Path(__file__).parent / "database_dumps" / database_dump
    _restore_database_dump(os.environ["DATABASE_URL"], database_file)


def _restore_database_dump(database_url: str, database_dump_path: Path) -> None:
    psql_path = shutil.which("psql")
    assert psql_path is not None
    result = subprocess.run([psql_path, "-f", str(database_dump_path), database_url], capture_output=True)
    if result.returncode != 0:
        print(result.stderr)
        raise RuntimeError(f"Could not restore database dump. psql exited with return code {result.returncode}")


@pytest.fixture(scope="session")
def generated_api_client_module(request: pytest.FixtureRequest) -> "types.ModuleType":
    client_path = Path(__file__).parent / "__api_client"
    openapi_source_path = Path(__file__).parent.parent / "etc" / "spec" / "openapi.json"
    generator_config_path = Path(__file__).parent / "openapi_python_config.yaml"
    openapi_generator_executable = shutil.which("openapi-generator-cli")
    if not openapi_generator_executable:
        pytest.skip("openapi-generator-cli is not available in the PATH")
    subprocess.run(
        [
            openapi_generator_executable,
            "generate",
            "--generator-name",
            "python",
            "--output",
            client_path.resolve(),
            "--input-spec",
            openapi_source_path.resolve(),
            "--config",
            generator_config_path,
        ],
        check=True,
    )

    shutil.rmtree(client_path / "kueaplan_api_client" / "test")

    sys.path.append(str(client_path))
    import kueaplan_api_client

    return kueaplan_api_client


@pytest.fixture(scope="function")
def generated_api_client(generated_api_client_module: types.ModuleType) -> "ApiClientWrapper":
    return ApiClientWrapper(generated_api_client_module)


class ApiClientWrapper:
    def __init__(self, kueaplan_api_client: types.ModuleType):
        self.module = kueaplan_api_client
        self.client = self._create_api_client()

    def _create_api_client(self) -> "kueaplan_api_client.DefaultApi":  # type: ignore  # noqa: F821
        BASE_URL = "http://localhost:9099/api/v1"
        config = self.module.Configuration(host=BASE_URL)
        client = self.module.ApiClient(config)
        return self.module.DefaultApi(client)

    def login(self, event_id: int, passphrase: str) -> None:
        auth_response = self.client.authorize(event_id, self.module.AuthorizeRequest(passphrase=passphrase))
        self.client.api_client.configuration.api_key["sessionTokenAuth"] = auth_response.session_token
