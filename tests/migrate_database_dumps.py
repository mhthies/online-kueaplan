#!/usr/bin/env python3
"""A script for migrating each of the database dumps in the database_dumps/ directory by
* loading the dump into the test database
* running `diesel migration run` against the test database
* recreating the database dump from the test database.

Preliminaries:
* The database URL of the test database to be used needs to be provided in the `DATABASE_URL` environment variable.
  It's possible to provide this environment variable in a .env file in the current working directory.
  This is compatible with the way that kueaplan_server and the PyTest tests search for the database.
* If a .env file is used, the `python-dotenv` package needs to be installed in the Python environment running this
  script.
* The following executables need to be available in the PATH: `psql`, `pg_dump`, `diesel`.
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Optional


def main() -> int:
    database_url = get_database_url()
    if database_url is None:
        return 2
    dumps = list_dumps()
    diesel_workdir = (Path(__file__).parent.parent / "server").absolute()
    try:
        for dump in dumps:
            print(f"> loading {dump.name} ...", file=sys.stderr)
            load_dump(dump, database_url)
            print("> migrating ...", file=sys.stderr)
            migrate_database(diesel_workdir, database_url)
            print(f"> dumping into {dump.name} ...", file=sys.stderr)
            save_dump(dump, database_url)
    except subprocess.CalledProcessError as e:
        print(f"Failed to execute command '{' '.join(e.cmd)}' (exit code: {e.returncode})", file=sys.stderr)
        return 1
    except CommandNotAvailableException as e:
        print(str(e), file=sys.stderr)
        return 1
    return 0


def get_database_url() -> Optional[str]:
    try:
        import dotenv

        dotenv.load_dotenv()
    except ImportError:
        print(
            "'python-dotenv' package is not available in this Python environment. Ignoring .env files.", file=sys.stderr
        )
    try:
        return os.environ["DATABASE_URL"]
    except KeyError:
        print(
            "'DATABASE_URL' environment variable is not set.\n"
            "        Please set it to the test database connection URL, e.g. postgres://user:password@localhost/kueaplan-test",
            file=sys.stderr,
        )
        return None


def list_dumps() -> list[Path]:
    dumps_dir = Path(__file__).parent / "database_dumps"
    return [dump_path.absolute() for dump_path in dumps_dir.glob("*.sql")]


def load_dump(dump_path: Path, database_url: str) -> None:
    subprocess.run([get_command_path("psql"), "-f", str(dump_path), database_url], check=True)


def migrate_database(diesel_workdir: Path, database_url: str) -> None:
    subprocess.run(
        [get_command_path("diesel"), "migration", "run", "--database-url", database_url], check=True, cwd=diesel_workdir
    )


def save_dump(dump_path: Path, database_url: str) -> None:
    subprocess.run([get_command_path("pg_dump"), "-c", "-f", str(dump_path), "--no-owner", database_url], check=True)


def get_command_path(executable_name: str) -> str:
    exe_path = shutil.which(executable_name)
    if exe_path is None:
        raise CommandNotAvailableException(executable_name)
    return exe_path


class CommandNotAvailableException(RuntimeError):
    def __init__(self, command_name: str):
        super().__init__(f"'{command_name}' executable could not be found in PATH")


if __name__ == "__main__":
    sys.exit(main())
