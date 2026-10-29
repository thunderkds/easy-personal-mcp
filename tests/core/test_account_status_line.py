"""Account status line (HUN-115): email/plan from `.claude.json`, tokens and last model from the store,
plan limits from `easy-personal-limits.json`. Fixtures only: temp agent root, temp store."""

import itertools
import json
import time
from datetime import UTC, datetime, timedelta

import pytest
from starlette.testclient import TestClient

from easy_personal.core import accounts, config
from easy_personal.surfaces import api

KEYS = {"email", "plan", "organization", "last_model", "last_active_utc", "tokens_today", "tokens_7d", "limits"}
SECRET = "INVENTED_SECRET_115"
NOW = datetime(2026, 10, 29, 12, 0, tzinfo=UTC).astimezone()  # fixed "now" for the pure helper
TOKEN, PORT = "tok-115", 5115


@pytest.fixture
def root():
    root = config.agent_root().resolve()
    (root / ".claude").mkdir()
    (root / ".claude-work" / "projects").mkdir(parents=True)
    (root / ".codex").mkdir()
    (root / ".codex" / "config.toml").write_text("")
    return root


def _stored(ts: datetime) -> str:
    return ts.astimezone(UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


_offsets = itertools.count(1)


def add_event(conn, account_id, ts, tokens=(10, 20, 30, 40), model="m-1", message_id=None):
    offset = next(_offsets)
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id,"
        " project_path, model, message_id, input_tokens, output_tokens, cache_creation_input_tokens,"
        " cache_read_input_tokens, agent_account_id) VALUES ('m','claude_code','/x',1,?,0,?,'s','/w',?,?,?,?,?,?,?)",
        (offset, _stored(ts), model, message_id or f"msg{offset}", *tokens, account_id),
    )
    conn.commit()


def lines(conn):
    return {a["folder"].rsplit("/", 1)[1]: a["status_line"] for a in accounts.grouped(conn, "account")["accounts"]}


def ids(conn, name):
    return conn.execute("SELECT id FROM agent_accounts WHERE folder LIKE ?", (f"%/{name}",)).fetchone()[0]


def write_claude_json(folder, **oauth):
    (folder / ".claude.json").write_text(json.dumps({
        "oauthAccount": oauth,
        "userID": SECRET,
        "mcpServers": {"x": {"env": {"TOKEN": SECRET}}},
    }))


def test_every_account_has_the_status_line_keys_in_both_groupings(conn, root):
    for by, pick in (("account", lambda g: g["accounts"]), ("provider", lambda g: [a for v in g["providers"].values() for a in v])):
        rows = pick(accounts.grouped(conn, by))
        assert len(rows) == 3
        assert all(set(r["status_line"]) == KEYS for r in rows)


def test_email_plan_organization_come_from_claude_json_and_nothing_else(conn, root):
    write_claude_json(root / ".claude-work", emailAddress="me@example.invalid", billingType="stripe_subscription",
                      organizationName="Fixture Org")
    line = lines(conn)[".claude-work"]
    assert (line["email"], line["plan"], line["organization"]) == ("me@example.invalid", "stripe_subscription", "Fixture Org")
    assert SECRET not in json.dumps(accounts.grouped(conn, "account"))


def test_default_account_falls_back_to_the_claude_json_beside_its_folder(conn, root):
    (root / ".claude.json").write_text(json.dumps({"oauthAccount": {"emailAddress": "home@example.invalid"}}))
    assert lines(conn)[".claude"]["email"] == "home@example.invalid"
    write_claude_json(root / ".claude", emailAddress="inside@example.invalid")  # the folder's own file wins
    assert lines(conn)[".claude"]["email"] == "inside@example.invalid"


@pytest.mark.parametrize("content", [None, "not json", "[]", "{}", '{"oauthAccount": 5}', '{"oauthAccount": {"emailAddress": 7}}'])
def test_missing_or_unusable_claude_json_gives_nulls(conn, root, content):
    if content is not None:
        (root / ".claude-work" / ".claude.json").write_text(content)
    line = lines(conn)[".claude-work"]
    assert (line["email"], line["plan"], line["organization"]) == (None, None, None)


def test_codex_account_reads_no_claude_json(conn, root):
    write_claude_json(root / ".codex", emailAddress="codex@example.invalid")
    (root / ".codex" / "easy-personal-limits.json").write_text(
        json.dumps({"five_hour": {"used_percentage": 1, "resets_at": 4102444800}}))
    line = lines(conn)[".codex"]
    assert (line["email"], line["plan"], line["organization"], line["limits"]) == (None, None, None, None)


def test_missing_account_folder_gives_nulls_and_status_missing(conn, root):
    accounts.detect(conn)
    (root / ".claude-work" / "projects").rmdir()
    (root / ".claude-work").rmdir()
    row = next(a for a in accounts.grouped(conn, "account")["accounts"] if a["folder"].endswith(".claude-work"))
    assert row["status"] == "missing" and row["status_line"]["email"] is None and row["status_line"]["limits"] is None


def test_tokens_today_and_7d_sum_all_token_columns_per_account(conn, root):
    accounts.detect(conn)
    work, default = ids(conn, ".claude-work"), ids(conn, ".claude")
    add_event(conn, work, NOW - timedelta(hours=1), tokens=(10, 20, 30, 40))  # 100 today
    add_event(conn, work, NOW - timedelta(days=3), tokens=(5, 5, 20, 20))  # 50, 3 days ago
    add_event(conn, work, NOW - timedelta(days=8), tokens=(1000, 0, 0, 0))  # outside 7 days
    add_event(conn, default, NOW - timedelta(hours=2), tokens=(1, 1, 1, 1))
    add_event(conn, None, NOW - timedelta(hours=2), tokens=(2, 2, 2, 2))  # NULL account = default account
    got = accounts.status_lines(conn, accounts.list_accounts(conn), now=NOW)
    assert (got[work]["tokens_today"], got[work]["tokens_7d"]) == (100, 150)
    assert (got[default]["tokens_today"], got[default]["tokens_7d"]) == (12, 12)


def test_a_message_split_over_several_events_counts_once(conn, root):
    accounts.detect(conn)
    work = ids(conn, ".claude-work")
    add_event(conn, work, NOW - timedelta(hours=1), tokens=(10, 5, 0, 0), message_id="same")
    add_event(conn, work, NOW - timedelta(hours=1, minutes=-1), tokens=(10, 20, 0, 0), message_id="same")
    got = accounts.status_lines(conn, accounts.list_accounts(conn), now=NOW)
    assert got[work]["tokens_today"] == 30


def test_today_starts_at_local_midnight(conn, root):
    accounts.detect(conn)
    work = ids(conn, ".claude-work")
    midnight = NOW.replace(hour=0, minute=0, second=0, microsecond=0)
    add_event(conn, work, midnight + timedelta(minutes=1), tokens=(7, 0, 0, 0))
    add_event(conn, work, midnight - timedelta(minutes=1), tokens=(3, 0, 0, 0))
    got = accounts.status_lines(conn, accounts.list_accounts(conn), now=NOW)[work]
    assert (got["tokens_today"], got["tokens_7d"]) == (7, 10)


def test_last_model_and_last_active_come_from_the_newest_event(conn, root):
    accounts.detect(conn)
    work, default = ids(conn, ".claude-work"), ids(conn, ".claude")
    add_event(conn, work, NOW - timedelta(days=20), model="old")
    add_event(conn, work, NOW - timedelta(days=10), model="newest")
    add_event(conn, None, NOW - timedelta(days=1), model="null-account")
    add_event(conn, default, NOW - timedelta(days=2), model="default-own")
    got = accounts.status_lines(conn, accounts.list_accounts(conn), now=NOW)
    assert got[work]["last_model"] == "newest"
    assert got[work]["last_active_utc"] == _stored(NOW - timedelta(days=10))
    assert got[default]["last_model"] == "null-account"  # the newer of its own and NULL-account events
    codex = next(a["id"] for a in accounts.list_accounts(conn) if a["provider"] == "codex")
    assert got[codex]["last_model"] is None and got[codex]["tokens_7d"] == 0


def test_account_with_no_events_has_null_model_and_zero_tokens(conn, root):
    line = lines(conn)[".claude-work"]
    assert (line["last_model"], line["last_active_utc"], line["tokens_today"], line["tokens_7d"]) == (None, None, 0, 0)


def write_limits(folder, payload):
    (folder / "easy-personal-limits.json").write_text(payload if isinstance(payload, str) else json.dumps(payload))


def test_limits_parse_epoch_to_iso_utc_and_are_fresh_when_reset_is_ahead(conn, root):
    reset = int((NOW + timedelta(hours=1)).timestamp())
    written = int(NOW.timestamp())
    write_limits(root / ".claude-work", {"five_hour": {"used_percentage": 42.5, "resets_at": reset},
                                         "seven_day": {"used_percentage": 10, "resets_at": reset + 86400},
                                         "written_at": written, "ignored": SECRET})
    accounts.detect(conn)
    limits = accounts.status_lines(conn, accounts.list_accounts(conn), now=NOW)[ids(conn, ".claude-work")]["limits"]
    assert limits["five_hour"] == {"used_percentage": 42.5,
                                   "resets_at": datetime.fromtimestamp(reset, UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
                                   "stale": False}
    assert limits["seven_day"]["used_percentage"] == 10 and limits["seven_day"]["stale"] is False
    assert limits["written_at"] == datetime.fromtimestamp(written, UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    assert limits["stale"] is False
    assert SECRET not in json.dumps(limits)


def test_a_past_reset_is_stale(conn, root):
    past = int((NOW - timedelta(minutes=1)).timestamp())
    future = int((NOW + timedelta(days=1)).timestamp())
    write_limits(root / ".claude-work", {"five_hour": {"used_percentage": 80, "resets_at": past},
                                         "seven_day": {"used_percentage": 20, "resets_at": future}, "written_at": past})
    accounts.detect(conn)
    limits = accounts.status_lines(conn, accounts.list_accounts(conn), now=NOW)[ids(conn, ".claude-work")]["limits"]
    assert (limits["five_hour"]["stale"], limits["seven_day"]["stale"], limits["stale"]) == (True, False, True)


@pytest.mark.parametrize("payload", [None, "{broken", "[]", "{}", '{"five_hour": 3}', '{"five_hour": {"used_percentage": "42"}}'])
def test_broken_or_empty_limits_file_gives_null(conn, root, payload):
    if payload is not None:
        write_limits(root / ".claude-work", payload)
    assert lines(conn)[".claude-work"]["limits"] is None


def test_percent_keeps_a_number_as_given_and_drops_other_types(conn, root):
    future = int((NOW + timedelta(hours=1)).timestamp())
    write_limits(root / ".claude-work", {"five_hour": {"used_percentage": 140, "resets_at": future},
                                         "seven_day": {"used_percentage": True, "resets_at": "soon"}})
    accounts.detect(conn)
    limits = accounts.status_lines(conn, accounts.list_accounts(conn), now=NOW)[ids(conn, ".claude-work")]["limits"]
    assert limits["five_hour"]["used_percentage"] == 140
    assert limits["seven_day"] is None and limits["written_at"] is None


def test_api_accounts_returns_status_line_for_both_groupings_and_never_errors(root):
    write_claude_json(root / ".claude-work", emailAddress="api@example.invalid", billingType="max", organizationName="Org")
    write_limits(root / ".claude-work", "{broken")
    client = TestClient(api.create_app(TOKEN, PORT), base_url=f"http://127.0.0.1:{PORT}")
    headers = {api.TOKEN_HEADER: TOKEN}
    flat = client.get("/api/accounts?by=account", headers=headers)
    assert flat.status_code == 200
    work = next(a for a in flat.json()["accounts"] if a["folder"].endswith(".claude-work"))
    assert work["status_line"]["email"] == "api@example.invalid" and work["status_line"]["limits"] is None
    by_provider = client.get("/api/accounts?by=provider", headers=headers).json()["providers"]
    assert all(set(a["status_line"]) == KEYS for rows in by_provider.values() for a in rows)
    assert SECRET not in flat.text


@pytest.mark.perf
def test_accounts_over_a_million_events_answer_under_100ms(conn, root):
    accounts.detect(conn)
    work, default = ids(conn, ".claude-work"), ids(conn, ".claude")
    start = datetime.now(UTC) - timedelta(days=365)
    step = 365 * 86400 / 1_000_000

    def rows():
        for i in range(1_000_000):
            stamp = _stored(start + timedelta(seconds=i * step))
            yield (i, stamp, f"s{i % 50}", f"msg{i}", (work, default, None)[i % 3])

    conn.executemany(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, ts_utc, session_id,"
        " project_path, model, message_id, input_tokens, output_tokens, cache_creation_input_tokens,"
        " cache_read_input_tokens, agent_account_id) VALUES ('m','claude_code','/x',1,?,0,?,?,'/w','m-1',?,10,5,0,0,?)",
        rows(),
    )
    conn.commit()
    assert conn.execute("SELECT COUNT(*) FROM events").fetchone()[0] == 1_000_000
    timings = []
    for _ in range(5):
        t = time.perf_counter()
        out = accounts.grouped(conn, "account")
        timings.append(time.perf_counter() - t)
    print(f"perf /api/accounts core over 1M events: {[round(x * 1000, 1) for x in timings]} ms")
    assert all(a["status_line"]["tokens_7d"] > 0 for a in out["accounts"] if a["provider"] == "claude")
    assert min(timings) < 0.1, timings
