"""Ordered store migrations. Module `mNNNN_<name>` exposes `apply(conn)`; NNNN is its `user_version`."""

import importlib
import pkgutil
import re

MIGRATIONS: list = sorted(
    (int(match[1]), importlib.import_module(f"{__name__}.{info.name}").apply)
    for info in pkgutil.iter_modules(__path__)
    if (match := re.fullmatch(r"m(\d{4})_\w+", info.name))
)
