"""The one secret check (FR-076): shared by saved reports, live labels and the transcript reader (HUN-235).

It lives in its own module with no imports but `re` so that `sources/claude_code.py` can use it without the
import cycle `reports` -> `rules` -> `kit` -> `sources.ui` -> `sources.claude_code`.
"""

import re

# Secret-looking text: key prefixes, a PEM header, or a 32+ character letter-and-digit run.
# A UUID (hyphens split it into runs of at most 12) and a file path (slashes split it) never match.
SECRET = re.compile(
    r"\bsk-[A-Za-z0-9]"
    r"|\bgh[pousr]_[A-Za-z0-9]|\bgithub_pat_"
    r"|\bxox[A-Za-z]?[-_]?[A-Za-z0-9]{4,}"
    r"|\b(?:AKIA|ASIA)[A-Z0-9]{4,}"
    r"|-----BEGIN"
    r"|[A-Za-z0-9]{32,}"
)
