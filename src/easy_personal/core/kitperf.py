"""Kit performance (FR-060..FR-063): sessions that used a kit MCP server, compared like with like.

One row per session (with at least one tool call) is built from `events` on every call (nothing is
stored). For each kit server and each measure (tokens per call, cost per call, good-result share), kit
sessions are compared with non-kit sessions inside strata of (owner project, main model, tool-call
band); the strata are pooled and a pooled difference is shown only when each side has at least 10
sessions. A difference of at least 5 % where the server side is better is a *finding* with a habit; the others are
listed as `other`. All say "linked to", never "caused". Habit texts are a fixed table here.
The rules are fixed in `PROJECT_SPEC.md` under Kit performance. Metadata only: no id, path or text
from an event leaves this module.
"""

import re
import sqlite3
import subprocess
import time
from bisect import bisect_left
from collections import Counter
from datetime import UTC, datetime
from pathlib import Path
from statistics import median

from easy_personal.core import config, discovery, metrics, owners, rules
from easy_personal.core import prices as price_table

SERVERS = ("easy-ui", "easy-verifier", "easy-context", "easy-memory")
MIN_SIDE = 10  # pooled sessions needed on each side of a finding (FR-062)
MIN_GAP_PCT = 5.0  # a smaller gap never carries a habit (owner, 2026-11-03)
DEFAULT_LIMIT, MAX_LIMIT = 3, 10
WINDOW_AFTER_SECONDS = 30 * 60  # a commit counts until 30 min after the last event (FR-061)
GIT_BUDGET_SECONDS = 2.0  # total git time per call (NFR-014)
NOTE = "linked to, not caused"
HABITS = {
    "easy-ui": "Check UI changes with easy-ui before you call the work done.",
    "easy-verifier": "Run easy-verifier on your change before you hand it over.",
    "easy-context": "Search with easy-context before you read files one by one.",
    "easy-memory": "Search easy-memory for earlier notes before you start the task.",
}
# measure -> (lower is better, words after "fewer|more" in the finding text)
MEASURES = {
    "tokens_per_call": (True, "tokens per tool call"),
    "cost_per_call": (True, "dollars of API-equivalent cost per tool call"),
    "good_result": (False, "good results"),
}
_BAD_REF = re.compile(r"[\s~^:?*\[\\\x00-\x1f\x7f]|\.\.|@\{|^-")  # never let a branch text act as a git option or range
_SHA = re.compile(r"^[0-9a-f]{40,64}$")


def _tool_servers(tool_names: str) -> set[str]:
    names = set(tool_names.split(","))
    return {s for s in SERVERS if any(n.startswith(f"mcp__{s}__") for n in names)}


def _epoch(stamp: str) -> float:
    return datetime.fromisoformat(stamp).timestamp()


# ---------------------------------------------------------------------------------------- git


class _Git:
    """Commit times per (repo, branch) with one time budget for the whole call; what it cannot check is unknown.

    `want` holds, per (repo, branch), the widest window any session needs, so git lists only those commits.
    """

    def __init__(self, budget: float, want: dict[tuple[str, str], tuple[int, int]]):
        self.deadline = time.monotonic() + budget
        self.want = want
        self.times: dict[tuple[str, str], list[int] | None] = {}
        self.merges: dict[str, dict[str, list[str]]] = {}

    def _rev_list(self, repo: str, *args: str) -> list[str] | None:
        left = self.deadline - time.monotonic()
        if left <= 0:
            return None
        try:
            out = discovery.run_git(repo, "rev-list", *args, timeout=left)
        except subprocess.TimeoutExpired:
            return None
        return out.splitlines() if out else None

    def _merge_shas(self, repo: str, branch: str) -> list[str]:
        if repo not in self.merges:
            found: dict[str, list[str]] = {}
            lines = self._rev_list(repo, "--all", "--merges", "--format=%s") or []
            sha = None
            for line in lines:  # `commit <sha>` then the subject
                if line.startswith("commit "):
                    sha = line[7:].strip()
                elif sha and _SHA.match(sha) and (named := owners.merge_branch(line)):
                    found.setdefault(named, []).append(sha)
            self.merges[repo] = found
        return self.merges[repo].get(branch, [])

    def commit_times(self, repo: str, branch: str) -> list[int] | None:
        """Sorted committer times of the commits on `branch` (through its merge commit when the ref is gone); None = unknown."""
        key = (repo, branch)
        if key not in self.times:
            self.times[key] = self._read(repo, branch)
        return self.times[key]

    def _read(self, repo: str, branch: str) -> list[int] | None:
        if _BAD_REF.search(branch):
            return None
        low, high = self.want[(repo, branch)]
        bounds = (f"--max-age={low}", f"--min-age={high}", "--timestamp")
        lines = self._rev_list(repo, *bounds, f"refs/heads/{branch}")
        if lines is None:  # the ref is gone, or it has no commit in the window: ask git whether the ref exists
            if self._rev_list(repo, "--max-count=1", f"refs/heads/{branch}") is not None:
                return []
            shas = self._merge_shas(repo, branch)
            if not shas:
                return None
            lines = []
            for sha in shas:
                lines += self._rev_list(repo, *bounds, f"{sha}^1..{sha}^2") or []
        return sorted(int(line.split()[0]) for line in lines if line.split() and line.split()[0].isdigit())


def _has_commit(times: list[int], low: int, high: int) -> bool:
    """A commit time in [low, high); `high` is last event + 30 min + 1 s, so 30 min after the last event still counts."""
    i = bisect_left(times, low)
    return i < len(times) and times[i] < high


# ----------------------------------------------------------------------------------- sessions


def _session_rows(conn: sqlite3.Connection, since_utc: str | None, prices) -> dict[str, dict]:
    """One dict per compared session: abstained = 0, at least one assistant event, a main model."""
    where, params = metrics._period("ts_utc", since_utc, None)
    models: dict[str, Counter] = {}
    rows: dict[str, dict] = {}
    for session, model, i, o, cc, cr in conn.execute(
        "SELECT session_id, MAX(model), MAX(input_tokens), MAX(output_tokens), MAX(cache_creation_input_tokens),"
        " MAX(cache_read_input_tokens) FROM events WHERE abstained = 0 AND session_id IS NOT NULL"
        f" AND message_id IS NOT NULL AND input_tokens IS NOT NULL AND {where} GROUP BY session_id, message_id",
        params,
    ):
        row = rows.setdefault(session, {"tokens": 0, "cost": 0.0})
        i, o, cc, cr = i or 0, o or 0, cc or 0, cr or 0
        row["tokens"] += i + o + cc + cr
        usd = prices.cost(model, i, o, cc, cr)
        row["cost"] = None if usd is None or row["cost"] is None else row["cost"] + usd
        if model:
            models.setdefault(session, Counter())[model] += o
    paths: dict[str, Counter] = {}
    branches: dict[str, Counter] = {}
    for session, path, branch, n, assistants, first, last, errors, known, tools in conn.execute(
        "SELECT session_id, project_path, git_branch, COUNT(*), SUM(record_type = 'assistant'), MIN(ts_utc),"
        " MAX(ts_utc), SUM(tool_error_count), COUNT(tool_error_count), GROUP_CONCAT(tool_names)"
        " FROM events WHERE abstained = 0 AND session_id IS NOT NULL AND ts_utc IS NOT NULL"
        f" AND {where} GROUP BY session_id, project_path, git_branch",
        params,
    ):
        row = rows.get(session)
        if row is None or session not in models:
            continue
        row["assistants"] = row.get("assistants", 0) + (assistants or 0)
        row["first"] = min(row.get("first", first), first)
        row["last"] = max(row.get("last", last), last)
        row["err_sum"] = row.get("err_sum", 0) + (errors or 0)
        row["err_known"] = row.get("err_known", 0) + known  # user records that carry a count (FR-059)
        row["calls"] = row.get("calls", 0) + (len(tools.split(",")) if tools else 0)
        row.setdefault("servers", set()).update(_tool_servers(tools) if tools else ())
        if path:
            paths.setdefault(session, Counter())[path] += n
        if branch:
            branches.setdefault(session, Counter())[branch] += n
    result = {}
    for session, row in rows.items():
        if not row.get("assistants") or session not in models or not row["calls"]:
            continue  # no assistant event, no model, or no tool call: not compared
        row["errors"] = row["err_sum"] if row["err_known"] else None  # NULL-aware SUM: no counted record = unknown
        row["tokens_per_call"] = row["tokens"] / row["calls"]
        row["cost_per_call"] = None if row["cost"] is None else row["cost"] / row["calls"]
        row["model"] = min(models[session].items(), key=lambda kv: (-kv[1], kv[0]))[0]
        row["path"] = min(paths[session].items(), key=lambda kv: (-kv[1], kv[0]))[0] if session in paths else None
        top = branches.get(session, Counter()).most_common(2)
        row["branch"] = top[0][0] if top and (len(top) == 1 or top[0][1] > top[1][1]) else None
        row["tie"] = len(top) == 2 and top[0][1] == top[1][1]
        result[session] = row
    return result


def _error_rate(row: dict) -> float | None:
    """Failed tool results / tool calls (every compared session has calls); no counted user record = unknown (never 0)."""
    return None if row["errors"] is None else row["errors"] / row["calls"]


def _candidate(row: dict, owner: dict[str, str]) -> tuple[str, str] | None:
    """(repo, branch) whose commits can tell this session's result; None = the result is unknown without git."""
    path = owner.get(row["path"], row["path"]) if row["path"] else None
    if _error_rate(row) is None or row["tie"] or not path or row["branch"] in (None, *owners.IGNORED_BRANCHES):
        return None
    return (path, row["branch"]) if (Path(path) / ".git").exists() else None


def _window(row: dict) -> tuple[int, int]:
    return int(_epoch(row["first"])), int(_epoch(row["last"])) + WINDOW_AFTER_SECONDS + 1


def _tertile_bands(sessions: dict[str, dict]) -> None:
    calls = sorted(r["calls"] for r in sessions.values())
    low, high = (calls[len(calls) // 3], calls[2 * len(calls) // 3]) if calls else (0, 0)
    for row in sessions.values():
        row["band"] = 0 if row["calls"] < low else 1 if row["calls"] < high else 2


def _prepare(conn: sqlite3.Connection, since_utc: str | None, prices, threshold: float, budget: float) -> dict[str, dict]:
    sessions = _session_rows(conn, since_utc, prices)
    _tertile_bands(sessions)
    owner = owners.owner_map(conn)
    want: dict[tuple[str, str], tuple[int, int]] = {}
    for row in sessions.values():
        row["stratum"] = (owner.get(row["path"], row["path"]), row["model"], row["band"])
        row["candidate"] = _candidate(row, owner)
        if row["candidate"]:
            low, high = _window(row)
            old = want.get(row["candidate"], (low, high))
            want[row["candidate"]] = (min(old[0], low), max(old[1], high))
    git = _Git(budget, want)
    for row in sorted(sessions.values(), key=lambda r: r["last"], reverse=True):  # newest first if the budget runs out
        times = git.commit_times(*row["candidate"]) if row["candidate"] else None
        if times is None:
            row["good_result"] = None
        else:
            low, high = _window(row)
            row["good_result"] = int(_has_commit(times, low, high) and _error_rate(row) <= threshold)  # a candidate has a known rate
    return sessions


def median_error_rate(conn: sqlite3.Connection) -> float | None:
    """Median error rate over sessions with at least one tool call and a known rate (used once, to set the default)."""
    rates = [r for row in _session_rows(conn, None, price_table.load()).values() if (r := _error_rate(row)) is not None]
    return median(rates) if rates else None


# -------------------------------------------------------------------------------- comparison


def _pool(sessions: dict[str, dict], server: str, measure: str) -> tuple[int, int, float | None]:
    """(kit sessions, other sessions, pooled delta %) over the strata that have both sides."""
    strata: dict[tuple, tuple[list[float], list[float]]] = {}
    for row in sessions.values():
        value = row[measure]
        if value is not None:
            strata.setdefault(row["stratum"], ([], []))[server not in row["servers"]].append(value)
    with_n = without_n = 0
    num = den = 0.0
    for kit_values, other_values in strata.values():
        if not kit_values or not other_values:
            continue
        other_mean = sum(other_values) / len(other_values)
        if other_mean == 0:
            continue
        with_n, without_n = with_n + len(kit_values), without_n + len(other_values)
        num += len(kit_values) * (sum(kit_values) / len(kit_values) - other_mean)
        den += len(kit_values) * other_mean
    return with_n, without_n, (num / den * 100 if den else None)


def _finding(server: str, measure: str, delta: float, with_n: int, without_n: int) -> dict:
    lower_is_better, words = MEASURES[measure]
    delta = round(delta, 1)
    text = (
        f"Sessions that used {server} are linked to {abs(delta):.1f}% {'fewer' if delta < 0 else 'more'} "
        f"{words} than similar sessions without it ({with_n} vs {without_n} sessions)."
    )
    better = (delta < 0 if lower_is_better else delta > 0) and abs(delta) >= MIN_GAP_PCT
    return {
        "server": server,
        "measure": measure,
        "delta_pct": delta,
        "with": with_n,
        "without": without_n,
        "text": text,
        "habit": HABITS[server] if better else None,
    }


def kit_findings(
    conn: sqlite3.Connection,
    limit: int = DEFAULT_LIMIT,
    since: str | None = None,
    *,
    prices: price_table.PriceTable | None = None,
    threshold: float | None = None,
) -> dict:
    """`findings` (server side better, habit, at most `limit`, capped at 10) and `other` (every other passing comparison)."""
    limit = max(1, min(limit, MAX_LIMIT))
    if threshold is None:
        threshold = rules.kit_error_rate(config.config_path())
    sessions = _prepare(conn, metrics._utc_bound(since), prices or price_table.load(), threshold, GIT_BUDGET_SECONDS)
    found, not_enough = [], []
    for server in SERVERS:
        with_n, without_n, _ = _pool(sessions, server, "tokens_per_call")
        if with_n < MIN_SIDE or without_n < MIN_SIDE:
            not_enough.append({"server": server, "with": with_n, "without": without_n})
            continue
        for measure in MEASURES:
            with_n, without_n, delta = _pool(sessions, server, measure)
            if delta is not None and with_n >= MIN_SIDE and without_n >= MIN_SIDE:
                found.append(_finding(server, measure, delta, with_n, without_n))
    found.sort(key=lambda f: (-abs(f["delta_pct"]), f["server"], f["measure"]))
    findings = [f for f in found if f["habit"]]
    return {
        "as_of": datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "sessions": len(sessions),
        "findings": findings[:limit],
        "other": [f for f in found if not f["habit"]],
        "not_enough": not_enough,
        "unknown_results": sum(r["good_result"] is None for r in sessions.values()),
        "note": NOTE,
    }
