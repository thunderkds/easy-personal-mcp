import pytest

from easy_personal.core import schedule
from easy_personal.surfaces import cli


@pytest.fixture
def xdg(tmp_path, monkeypatch):
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / "xdg"))
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    return tmp_path / "xdg" / "systemd" / "user"


@pytest.fixture
def systemctl(monkeypatch):
    calls = []
    monkeypatch.setattr(schedule, "systemd_available", lambda: True)
    monkeypatch.setattr(schedule, "systemctl", lambda *a: calls.append(a))
    return calls


def test_render_units_use_absolute_command_and_persistent():
    units = schedule.render_units("/opt/bin/easy-personal", retention_days=None)
    assert "ExecStart=/opt/bin/easy-personal ingest\n" in units[schedule.SERVICE]
    assert "--retention-days" not in units[schedule.SERVICE]
    assert "Persistent=true" in units[schedule.TIMER]
    assert "WantedBy=timers.target" in units[schedule.TIMER]
    with_ret = schedule.render_units("/x/ep", retention_days=90)
    assert "ExecStart=/x/ep ingest --retention-days 90\n" in with_ret[schedule.SERVICE]


def test_install_writes_units_and_enables_timer(xdg, systemctl, capsys):
    assert cli.main(["schedule", "install", "--retention-days", "60"]) == 0
    assert (xdg / schedule.SERVICE).read_text().count("--retention-days 60") == 1
    assert (xdg / schedule.TIMER).exists()
    assert ("enable", "--now", schedule.TIMER) in systemctl


def test_install_twice_rewrites(xdg, systemctl):
    cli.main(["schedule", "install"])
    (xdg / schedule.SERVICE).write_text("stale")
    cli.main(["schedule", "install"])
    assert "ExecStart=" in (xdg / schedule.SERVICE).read_text()


def test_remove_reverses_install(xdg, systemctl):
    cli.main(["schedule", "install"])
    assert cli.main(["schedule", "remove"]) == 0
    assert not (xdg / schedule.SERVICE).exists()
    assert not (xdg / schedule.TIMER).exists()
    assert ("disable", "--now", schedule.TIMER) in systemctl


def test_remove_when_nothing_installed_is_ok(xdg, systemctl):
    assert cli.main(["schedule", "remove"]) == 0


def test_no_systemd_prints_cron_snippet_and_writes_nothing(xdg, monkeypatch, capsys):
    monkeypatch.setattr(schedule, "systemd_available", lambda: False)
    assert cli.main(["schedule", "install", "--retention-days", "7"]) != 0
    out = capsys.readouterr().out
    assert "crontab" in out and " ingest --retention-days 7" in out
    assert not xdg.exists()


def test_dry_run_prints_units_and_writes_nothing(xdg, monkeypatch, capsys):
    monkeypatch.setattr(schedule, "systemd_available", lambda: False)
    assert cli.main(["schedule", "install", "--dry-run"]) == 0
    out = capsys.readouterr().out
    assert schedule.SERVICE in out and schedule.TIMER in out
    assert not xdg.exists()
    assert cli.main(["schedule", "remove", "--dry-run"]) == 0
