import json
import os
import sys
import types
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


@pytest.fixture(scope="session", autouse=True)
def start_kueaplan_server(request: pytest.FixtureRequest, load_dotenv: None):
    if not request.config.getoption("--start-app"):
        yield
        return
    if shutil.which("cargo"):
        executable_path = _cargo_build_and_get_executable_path(Path(__file__).parent.parent / "server")
    else:
        # best guess: Is already built and located at ../../target/debug/kueaplan_server
        path = (Path(__file__).parent.parent / "target" / "debug" / "kueaplan_server").resolve()
        if not path.is_file():
            raise RuntimeError("Could not find pre-compiled kueaplan_server. Run without --start-app and execute the server manually.")
        executable_path = str(path)

    _restore_database_dump(os.environ["DATABASE_URL"], Path(__file__).parent / "database_dumps" / "minimal.sql")

    cmd = [executable_path, "serve"]
    env = dict(os.environ)
    env["LISTEN_PORT"] = "9099"
    env["LISTEN_ADDRESS"] = "127.0.0.1"
    env["ADMIN_NAME"] = "Anton Administrator"
    env["ADMIN_EMAIL"] = "anton@example.com"
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
def load_dotenv():
    dotenv.load_dotenv()


@pytest.fixture(scope="function")
def reset_database(request: pytest.FixtureRequest, load_dotenv: None):
    database_dump = request.node.get_closest_marker("database_dump")
    if database_dump is None:
        database_dump = "minimal.sql"
    database_file = Path(__file__).parent / "database_dumps" / database_dump
    _restore_database_dump(os.environ["DATABASE_URL"], database_file)


def _restore_database_dump(database_url: str, database_dump_path: Path) -> None:
    result = subprocess.run([shutil.which("psql"), "-f", str(database_dump_path), database_url],
                            capture_output=True)
    if result.returncode != 0:
        print(result.stderr)
        raise RuntimeError(f"Could not restore database dump. psql exited with return code {result.returncode}")


@pytest.fixture(scope="session")
def generated_api_client(request: pytest.FixtureRequest) -> "ApiClientWrapper":
    client_path = Path(__file__).parent / "__api_client"
    openapi_source_path = Path(__file__).parent.parent / "etc" / "spec" / "openapi.json"
    generator_config_path = Path(__file__).parent / "openapi_python_config.yaml"
    openapi_generator_executable = shutil.which("openapi-generator-cli")
    if not openapi_generator_executable:
        pytest.skip("openapi-generator-cli is not available in the PATH")
    subprocess.run([openapi_generator_executable,
                    "generate",
                    "--generator-name", "python",
                    "--output", client_path.resolve(),
                    "--input-spec", openapi_source_path.resolve(),
                    "--config", generator_config_path],
                   check=True)

    shutil.rmtree(client_path / "kueaplan_api_client" / "test")

    sys.path.append(str(client_path))
    import kueaplan_api_client
    return ApiClientWrapper(kueaplan_api_client)


class ApiClientWrapper:
    def __init__(self, kueaplan_api_client: types.ModuleType):
        self.module = kueaplan_api_client
        self.client = self._create_api_client()

    def _create_api_client(self) -> "kueaplan_api_client.DefaultApi":
        BASE_URL = "http://localhost:9099/api/v1"
        config = self.module.Configuration(host=BASE_URL)
        client = self.module.ApiClient(config)
        return self.module.DefaultApi(client)

    def login(self, event_id: int, passphrase: str) -> None:
        auth_response = self.client.authorize(event_id, self.module.AuthorizeRequest(passphrase=passphrase))
        self.client.api_client.configuration.api_key["sessionTokenAuth"] = auth_response.session_token
