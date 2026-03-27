#!/usr/bin/env python3
import errno
import fcntl
import os
import pty
import select
import signal
import socket
import struct
import subprocess
import sys
import termios


def fail(message: str, code: int = 64) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(code)


def choose_command(repo_root: str) -> tuple[list[str], str]:
    rust_dir = os.path.join(repo_root, "rust")
    release_bin = os.path.join(rust_dir, "target", "release", "ec-game")
    debug_bin = os.path.join(rust_dir, "target", "debug", "ec-game")
    if os.path.isfile(release_bin) and os.access(release_bin, os.X_OK):
        return [release_bin], repo_root
    if os.path.isfile(debug_bin) and os.access(debug_bin, os.X_OK):
        return [debug_bin], repo_root
    return ["cargo", "run", "-q", "-p", "ec-game", "--"], rust_dir


def set_winsize(fd: int, cols: int, rows: int) -> None:
    packed = struct.pack("HHHH", rows, cols, 0, 0)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, packed)


def main() -> int:
    if len(sys.argv) < 6:
        fail(
            "usage: run_ec_rust_bridge.py <game_dir> <dropfile_path> <srv_port> <term_width> <term_height> [extra ec-game args...]"
        )

    game_dir = sys.argv[1]
    dropfile = sys.argv[2]
    srv_port = int(sys.argv[3])
    term_width = int(sys.argv[4])
    term_height = int(sys.argv[5])
    extra_args = sys.argv[6:]

    if not os.path.isdir(game_dir):
        fail(f"ec-game bridge error: game dir not found: {game_dir}", 66)
    if not os.path.isfile(dropfile):
        fail(f"ec-game bridge error: dropfile not found: {dropfile}", 66)

    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    base_cmd, cwd = choose_command(repo_root)

    export_root = os.environ.get("EC_CLIENT_EXPORT_ROOT")
    if not export_root:
        export_root = os.path.join(game_dir, "exports")
        os.environ["EC_CLIENT_EXPORT_ROOT"] = export_root
    os.makedirs(export_root, exist_ok=True)

    queue_dir = os.environ.get("EC_CLIENT_QUEUE_DIR")
    if queue_dir:
        os.makedirs(queue_dir, exist_ok=True)

    cmd = base_cmd + [
        "--dir",
        game_dir,
        "--dropfile",
        dropfile,
        "--encoding",
        "cp437",
        "--color-mode",
        "ansi16",
        *extra_args,
    ]

    sock = socket.create_connection(("127.0.0.1", srv_port))
    sock.setblocking(False)

    master_fd, slave_fd = pty.openpty()
    set_winsize(slave_fd, term_width, term_height)

    child = subprocess.Popen(
        cmd,
        stdin=slave_fd,
        stdout=slave_fd,
        stderr=slave_fd,
        cwd=cwd,
        close_fds=True,
        env=os.environ.copy(),
        start_new_session=True,
    )
    os.close(slave_fd)

    def terminate_child() -> None:
        if child.poll() is not None:
            return
        try:
            os.killpg(child.pid, signal.SIGTERM)
        except ProcessLookupError:
            return

    try:
        while True:
            if child.poll() is not None:
                break
            readable, _, _ = select.select([sock, master_fd], [], [], 0.1)
            for source in readable:
                if source is sock:
                    try:
                        data = sock.recv(4096)
                    except BlockingIOError:
                        continue
                    if not data:
                        terminate_child()
                        return child.wait(timeout=2)
                    os.write(master_fd, data)
                else:
                    try:
                        data = os.read(master_fd, 4096)
                    except OSError as exc:
                        if exc.errno == errno.EIO:
                            data = b""
                        else:
                            raise
                    if not data:
                        return child.wait(timeout=2)
                    view = memoryview(data)
                    while view:
                        try:
                            written = sock.send(view)
                        except BlockingIOError:
                            select.select([], [sock], [], 0.1)
                            continue
                        view = view[written:]
        return child.wait(timeout=2)
    finally:
        terminate_child()
        try:
            sock.close()
        finally:
            try:
                os.close(master_fd)
            except OSError:
                pass


if __name__ == "__main__":
    raise SystemExit(main())
