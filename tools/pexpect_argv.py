from typing import Sequence

import pexpect


def spawn_argv(
    cmd: Sequence[str],
    *,
    env: dict[str, str] | None = None,
    timeout: int | float = 30,
    encoding: str = "cp437",
) -> pexpect.spawn:
    return pexpect.spawn(cmd[0], list(cmd[1:]), env=env, timeout=timeout, encoding=encoding)
