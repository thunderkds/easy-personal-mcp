"""Scheduled jobs reader (HUN-163, FR-040): systemd user timers + crontab, agent allow-list, first word only."""

import json
from datetime import datetime

import pytest

from easy_personal.core import config, jobs
from fakes import jobs_fakes
from fakes.jobs_fakes import exec_start

SECRET = "SECRET_X"
US = 1_000_000
NEXT = 1_793_444_400 * US  # 2026-10-31T11:00:00Z
LAST = 1_793_433_606 * US


def timer(unit, activates=None, nxt=NEXT, last=LAST):
    return {"unit": unit, "activates": activates or unit[:-6] + ".service", "next": nxt, "last": last}


def props(timer_unit, argv, calendar="daily", status="0"):
    service = timer_unit[:-6] + ".service"
    return {
        timer_unit: {"TimersCalendar": f"{{ OnCalendar={calendar} ; next_elapse=Sat 2026-10-31 18:00:00 +07 }}"},
        service: {"ExecStart": exec_start(argv), "ExecMainStatus": status},
    }


@pytest.fixture
def bindir(tmp_path, monkeypatch):
    """PATH holds only the fakes dir: the real systemctl and crontab are never reachable."""
    path = tmp_path / "fakebin"
    monkeypatch.setenv("PATH", str(path))
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    return path


def by_name(result):
    return {j["name"]: j for j in result["jobs"]}


def two_agent_one_other(bindir, **kw):
    timers = [timer("easy-personal-ingest.timer"), timer("tidy.timer")]
    p = {
        **props("easy-personal-ingest.timer", f"/home/x/.local/bin/easy-personal ingest --token {SECRET}"),
        **props("tidy.timer", f"/usr/bin/rm -rf /tmp/{SECRET}"),
    }
    cron = f'# my jobs\nMAILTO=me@example.test\n0 7 * * 1-5 claude -p "{SECRET}"\n'
    return jobs_fakes.install(bindir, timers=timers, props=p, crontab=cron, **kw)


# --- command_name: the first word, nothing else -----------------------------------------------


@pytest.mark.parametrize(
    "line, expected",
    [
        ("claude -p hello", "claude"),
        ("/home/x/.local/bin/claude -p hello", "claude"),
        ("env A=1 B=2 claude -p hello", "claude"),
        ("env -i A=1 /opt/bin/easy-personal ingest", "easy-personal"),
        ("A=1 claude run", "claude"),
        ("sh -c 'claude -p \"hello\"'", "claude"),
        ('bash -c "A=1 easy-personal ingest"', "easy-personal"),
        ("/usr/bin/python3 -m easy_personal.surfaces.cli ingest", "easy-personal"),
        ("cd /tmp && claude", "cd"),
        ("/usr/bin/python3 script.py", "python3"),
        ("", None),
        ("env A=1", None),
    ],
)
def test_command_name_keeps_only_the_first_word(line, expected):
    assert jobs.command_name(line) == expected


# --- AC1 + AC2: first word only, allow-list, hidden count -------------------------------------


def test_two_rows_one_hidden_and_no_argument_anywhere(bindir):
    two_agent_one_other(bindir)
    result = jobs.list_jobs()
    assert sorted(j["name"] for j in result["jobs"]) == ["(crontab) claude", "easy-personal-ingest.timer"]
    assert result["hidden"] == 1
    assert SECRET not in json.dumps(result)
    assert "--token" not in json.dumps(result) and "-rf" not in json.dumps(result)


def test_rows_carry_only_the_first_word(bindir):
    two_agent_one_other(bindir)
    rows = by_name(jobs.list_jobs())
    assert rows["easy-personal-ingest.timer"]["command"] == "easy-personal"
    assert rows["(crontab) claude"]["command"] == "claude"
    assert rows["(crontab) claude"]["kind"] == "cron"
    assert rows["(crontab) claude"]["schedule"] == "0 7 * * 1-5"


def test_unrelated_cron_line_is_hidden_not_listed(bindir):
    jobs_fakes.install(bindir, crontab=f"5 4 * * * /usr/bin/backup --key {SECRET}\n@reboot claude x\n")
    result = jobs.list_jobs()
    assert [j["name"] for j in result["jobs"]] == ["(crontab) claude"]
    assert result["hidden"] == 1


def test_allow_list_comes_from_config(bindir):
    jobs_fakes.install(bindir, crontab="5 4 * * * /usr/bin/backup now\n1 1 * * * claude x\n")
    (config.data_dir() / "config.yaml").write_text("jobs:\n  allow: backup, easy-personal\n")
    result = jobs.list_jobs()
    assert [j["command"] for j in result["jobs"]] == ["backup"]
    assert result["hidden"] == 1


def test_default_allow_list_names_the_agent_and_kit_tools(monkeypatch, tmp_path):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path))
    assert {"claude", "easy-personal", "easy-personal-launch"} <= set(config.jobs_allow())


# --- AC3: our own timer ------------------------------------------------------------------------


def test_own_ingest_timer_shows_next_last_and_exit_code(bindir):
    jobs_fakes.install(
        bindir,
        timers=[timer("easy-personal-ingest.timer")],
        props=props("easy-personal-ingest.timer", "/usr/bin/python3 -m easy_personal.surfaces.cli ingest", status="1"),
    )
    row = by_name(jobs.list_jobs())["easy-personal-ingest.timer"]
    assert row["kind"] == "systemd"
    assert row["schedule"] == "daily"
    assert row["next_run"] == "2026-10-31T11:00:00Z"
    assert row["last_run"] == "2026-10-31T08:00:06Z"
    assert row["last_exit"] == 1
    assert row["command"] == "easy-personal"


def test_timer_that_never_ran_has_no_last_run_and_no_exit_code(bindir):
    jobs_fakes.install(
        bindir,
        timers=[timer("easy-personal-ingest.timer", last=0)],
        props=props("easy-personal-ingest.timer", "/usr/bin/easy-personal ingest"),
    )
    row = by_name(jobs.list_jobs())["easy-personal-ingest.timer"]
    assert row["last_run"] is None and row["last_exit"] is None


def test_timer_without_a_service_is_hidden(bindir):
    jobs_fakes.install(bindir, timers=[timer("orphan.timer")], props={})
    result = jobs.list_jobs()
    assert result["jobs"] == [] and result["hidden"] == 1


def test_monotonic_timer_schedule(bindir):
    p = props("easy-personal-ingest.timer", "/usr/bin/easy-personal ingest")
    p["easy-personal-ingest.timer"] = {"TimersMonotonic": "{ OnBootSec=5min ; next_elapse=n/a } { OnUnitActiveSec=1h ; next_elapse=n/a }"}
    jobs_fakes.install(bindir, timers=[timer("easy-personal-ingest.timer")], props=p)
    assert by_name(jobs.list_jobs())["easy-personal-ingest.timer"]["schedule"] == "OnBootSec=5min, OnUnitActiveSec=1h"


def test_text_fallback_when_systemd_has_no_json_output(bindir):
    jobs_fakes.install(
        bindir,
        json_output=False,
        timers=[timer("easy-personal-ingest.timer")],
        props=props("easy-personal-ingest.timer", "/usr/bin/easy-personal ingest"),
    )
    result = jobs.list_jobs()
    row = by_name(result)["easy-personal-ingest.timer"]
    assert result["sources"]["systemd"] == "ok"
    assert row["schedule"] == "daily" and row["next_run"] is None and row["last_exit"] is None


# --- AC4: no data per kind ---------------------------------------------------------------------


def test_no_systemctl_is_no_data_and_cron_still_lists(bindir):
    jobs_fakes.install(bindir, crontab="0 7 * * * claude run\n", with_systemctl=False)
    result = jobs.list_jobs()
    assert result["sources"] == {"systemd": "no data", "cron": "ok"}
    assert [j["kind"] for j in result["jobs"]] == ["cron"]


def test_no_crontab_binary_is_no_data_and_timers_still_list(bindir):
    jobs_fakes.install(
        bindir,
        with_crontab=False,
        timers=[timer("easy-personal-ingest.timer")],
        props=props("easy-personal-ingest.timer", "/usr/bin/easy-personal ingest"),
    )
    result = jobs.list_jobs()
    assert result["sources"] == {"systemd": "ok", "cron": "no data"}
    assert [j["kind"] for j in result["jobs"]] == ["systemd"]


def test_empty_crontab_is_no_data(bindir):
    jobs_fakes.install(bindir, crontab=None)
    assert jobs.list_jobs() == {"jobs": [], "hidden": 0, "sources": {"systemd": "ok", "cron": "no data"}}


def test_no_user_bus_is_no_data(bindir):
    jobs_fakes.install(bindir, bus=False, crontab="0 7 * * * claude run\n")
    result = jobs.list_jobs()
    assert result["sources"]["systemd"] == "no data"
    assert len(result["jobs"]) == 1


def test_nothing_installed_is_no_data_for_both(bindir):
    bindir.mkdir()
    assert jobs.list_jobs() == {"jobs": [], "hidden": 0, "sources": {"systemd": "no data", "cron": "no data"}}


def test_a_hanging_tool_times_out_into_no_data(bindir, monkeypatch):
    monkeypatch.setattr(jobs, "TIMEOUT", 0.3)
    jobs_fakes.install(bindir, hang=3, crontab="0 7 * * * claude run\n")
    result = jobs.list_jobs()
    assert result["sources"] == {"systemd": "no data", "cron": "no data"}


# --- AC5: read commands only -------------------------------------------------------------------


def test_only_read_subcommands_are_ever_invoked(bindir):
    log = two_agent_one_other(bindir)
    jobs.list_jobs()
    seen = jobs_fakes.calls(log)
    assert seen, "the fakes were never called"
    for argv in seen:
        if argv[0] == "systemctl":
            assert argv[1] == "--user" and argv[2] in {"list-timers", "show"}, argv
        else:
            assert argv == ["crontab", "-l"], argv


def test_every_call_has_a_timeout():
    import inspect

    assert "timeout=TIMEOUT" in inspect.getsource(jobs._run)


@pytest.mark.parametrize(
    "argv",
    [
        ["systemctl", "--user", "start", "x.timer"],
        ["systemctl", "--user", "enable", "--now", "x.timer"],
        ["systemctl", "stop", "x.timer"],
        ["crontab", "-r"],
        ["crontab", "-e"],
        ["crontab", "file"],
        ["rm", "-rf", "/"],
    ],
)
def test_run_refuses_anything_but_the_read_commands(argv, bindir):
    jobs_fakes.install(bindir)
    with pytest.raises(ValueError):
        jobs._run(argv)


def test_a_unit_name_that_looks_like_an_option_is_never_passed_on(bindir):
    log = jobs_fakes.install(bindir, timers=[timer("--now.timer")], props={})
    jobs.list_jobs()
    assert not any("--now.timer" in a for argv in jobs_fakes.calls(log) for a in argv if a != "list-timers")


# --- cron lines --------------------------------------------------------------------------------


def test_cron_skips_comments_env_lines_and_blank_lines(bindir):
    jobs_fakes.install(bindir, crontab="# c\n\nPATH=/usr/bin\nSHELL=/bin/sh\n@hourly claude a\n@reboot easy-personal b\n")
    result = jobs.list_jobs()
    assert sorted(j["schedule"] for j in result["jobs"]) == ["@hourly", "@reboot"]
    assert result["hidden"] == 0


def test_cron_last_run_and_exit_are_unknown(bindir):
    jobs_fakes.install(bindir, crontab="0 7 * * * claude run\n")
    row = jobs.list_jobs()["jobs"][0]
    assert row["last_run"] is None and row["last_exit"] is None


def test_cron_next_run_is_reported_in_utc(bindir):
    jobs_fakes.install(bindir, crontab="*/15 * * * * claude run\n")
    nxt = jobs.list_jobs()["jobs"][0]["next_run"]
    assert nxt.endswith("Z") and datetime.strptime(nxt, "%Y-%m-%dT%H:%M:%SZ")


@pytest.mark.parametrize(
    "expr, after, expected",
    [
        ("0 7 * * 1-5", datetime(2026, 10, 31, 12, 0), datetime(2026, 11, 2, 7, 0)),  # Saturday -> Monday
        ("*/15 * * * *", datetime(2026, 10, 31, 12, 7, 30), datetime(2026, 10, 31, 12, 15)),
        ("30 3 1 * *", datetime(2026, 10, 31, 12, 0), datetime(2026, 11, 1, 3, 30)),
        ("0 0 * * 0", datetime(2026, 10, 31, 12, 0), datetime(2026, 11, 1, 0, 0)),  # Sunday = 0
        ("0 0 * * 7", datetime(2026, 10, 31, 12, 0), datetime(2026, 11, 1, 0, 0)),  # Sunday = 7
        ("0 9 1,15 * *", datetime(2026, 10, 31, 12, 0), datetime(2026, 11, 1, 9, 0)),
        ("0 0 29 2 *", datetime(2026, 10, 31, 12, 0), datetime(2028, 2, 29, 0, 0)),
        ("0 12 * * *", datetime(2026, 10, 31, 12, 0), datetime(2026, 11, 1, 12, 0)),  # strictly after
    ],
)
def test_next_cron(expr, after, expected):
    assert jobs.next_cron(expr, after) == expected


@pytest.mark.parametrize("expr", ["@reboot", "0 7 * * mon", "61 * * * *", "* * *", "*/0 * * * *", "5-1 * * * *", "x * * * *"])
def test_next_cron_unknown_for_what_it_cannot_read(expr):
    assert jobs.next_cron(expr, datetime(2026, 10, 31, 12, 0)) is None
