import subprocess
from pathlib import Path

from . import util


def create_passphrase(
    kueaplan_executable: Path, event_id_or_slug: str, role: str, passphrase: str, derivable_link_passphrase: bool = True
) -> None:
    cmd = [str(kueaplan_executable), "passphrase", "create", event_id_or_slug]
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stdin=subprocess.PIPE)
    assert process.stdout is not None
    try:
        util.wait_for_prompt_and_type(process, "access role", role)
        util.wait_for_prompt_and_type(process, "passphrase", passphrase)
        util.wait_for_prompt_and_type(process, "Comment", "")
        util.wait_for_prompt_and_type(process, "valid until", "")
        util.wait_for_prompt_and_type(
            process, "derivable passphrase for link-sharing", "y" if derivable_link_passphrase else "n"
        )
        process.wait(1)
        final_output = process.stdout.read()
        assert b"Success" in final_output
        if process.returncode:
            raise subprocess.CalledProcessError(process.returncode, cmd)
    finally:
        process.terminate()
        process.wait(1)
        process.kill()
