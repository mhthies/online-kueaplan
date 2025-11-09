import fcntl
import os
import subprocess
import time
from typing import IO, cast


def wait_for_interactive_prompt(std_out_stream: IO[bytes], timeout: float = 1.0) -> bytes:
    """Wait up to `timeout` seconds for the process to output the interactive prompt ('\n>'). Reads and returns all
    bytes from the stdout buffer up to the prompt.
    """
    make_pipe_non_blocking(std_out_stream)
    buffer = bytearray()
    tic = time.time()
    while True:
        if (output := std_out_stream.read()) is not None:
            buffer.extend(output)
        if b"\n>" in buffer:
            return buffer
        if time.time() - tic > timeout:
            raise TimeoutError(f"Subprocess did not output interactive prompt within {timeout}s. Stdout: {buffer!r}")
        time.sleep(0.02)


def make_pipe_non_blocking(pipe: IO) -> None:
    fd = pipe.fileno()
    fl = fcntl.fcntl(fd, fcntl.F_GETFL)
    fcntl.fcntl(fd, fcntl.F_SETFL, fl | os.O_NONBLOCK)


def wait_for_prompt_and_type(process: subprocess.Popen, expected_prompt: str, enter_input: str) -> None:
    assert process.stdout is not None
    assert process.stdin is not None
    output = wait_for_interactive_prompt(cast(IO[bytes], process.stdout))
    assert expected_prompt.encode() in output
    process.stdin.write(f"{enter_input}\n".encode())
    process.stdin.flush()
