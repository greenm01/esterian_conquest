from pathlib import Path


def _write_crlf_lines(path: str | Path, lines: list[str]) -> None:
    Path(path).write_bytes(("\r\n".join(lines) + "\r\n").encode("ascii"))


def write_chain_txt(
    path: str | Path,
    *,
    player_number: int = 1,
    first_name: str = "sysop",
    last_name: str = "Sysop",
    security_level: int = 25,
    ansi: str = "Y",
    remote: str = "N",
    columns: int = 80,
    rows: int = 24,
) -> None:
    # ECGAME's CHAIN.TXT parser proved sensitive to both record count and line endings.
    lines = [
        str(player_number),
        first_name,
        last_name,
        "1",
        str(security_level),
        ansi,
        remote,
        str(columns),
        str(rows),
    ] + ["1"] * 23
    _write_crlf_lines(path, lines)


def write_door_sys(
    path: str | Path,
    *,
    com_port: str = "COM1:",
    baud: int = 19200,
    data_bits: int = 8,
    parity: str = "N",
    stop_bits: int = 1,
    user_first_name: str = "Sysop",
    user_last_name: str = "HANNIBAL",
    location: str = "Orlando, FL",
    phone: str = "123-456-7890",
) -> None:
    lines = [
        com_port,
        str(baud),
        str(data_bits),
        parity,
        str(stop_bits),
        "1",
        "Y",
        "Y",
        "Y",
        "Y",
        user_first_name,
        user_last_name,
        location,
        phone,
        "1",
        "0",
        "100",
        "9000",
        "1",
        "99",
    ] + ["90"] * 36
    _write_crlf_lines(path, lines)
