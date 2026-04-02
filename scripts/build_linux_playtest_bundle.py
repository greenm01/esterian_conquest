#!/usr/bin/env python3
from __future__ import annotations

import build_release_bundle


if __name__ == "__main__":
    build_release_bundle.main(default_target="x86_64-unknown-linux-gnu")
