"""`GET /api/kit/reports` never shows a report file name, a sub-folder name, a report's content or a full path
(HUN-240, NFR-002, FR-089).
"""

from starlette.testclient import TestClient

from easy_personal.core import config, store, workspaces
from easy_personal.surfaces import api
from leak import harness
from leak.canaries import Canaries

TOKEN, PORT = "leak-240", 5541
URL = "/api/kit/reports"


def test_the_response_and_logs_hold_only_folder_names_numbers_and_times(tmp_path, monkeypatch):
    canaries = Canaries.new()
    root = tmp_path.resolve()
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(root / "home"))
    names = [v for v in canaries.values.values() if v.isprintable() and not set(v) & set("/\0")]
    name_canary, folder_canary = names[0], names[1]
    repo = root / "deep" / "tree" / "planted-repo"
    reports = repo / "reports"
    (reports / folder_canary / "inner").mkdir(parents=True)
    (reports / name_canary).write_text("\n".join(canaries.values.values()))  # a report file: planted NAME and CONTENT
    (reports / folder_canary / "inner" / "session-x.json").write_text("\n".join(canaries.values.values()))
    (repo / "docs").mkdir()  # not a report folder: its name never shows
    with store.opened(config.store_path()) as conn:
        workspaces.add(conn, "acme", description=canaries.values["api_key"], email=canaries.values["email"], repos=[str(repo)])
        conn.commit()
    client = TestClient(api.create_app(TOKEN, PORT), base_url=f"http://127.0.0.1:{PORT}")
    reports.joinpath("locked").mkdir()
    reports.joinpath("locked").chmod(0)  # an unreadable sub-folder: no error text with its path
    try:
        with harness.LogCapture() as logs:
            res = client.get(URL, headers={api.TOKEN_HEADER: TOKEN})
            bodies = [f"{res.status_code} {dict(res.headers)} {res.text}"]
            log_text = logs.text()
    finally:
        reports.joinpath("locked").chmod(0o700)
    (acme,) = res.json()["workspaces"]
    (folder,) = acme["folders"]
    assert (folder["repo"], folder["folder"], folder["files"]) == ("planted-repo", "reports", 2)  # the response really carries data
    assert folder["easy_ui_sessions"] == 1
    assert set(res.json()) == {"workspaces"} and set(acme) == {"name", "folders"}
    for blob in (*bodies, log_text):
        canaries.assert_clean(URL, blob)
        assert name_canary not in blob and folder_canary not in blob
        assert str(root) not in blob and "deep/tree" not in blob and "inner" not in blob and "docs" not in blob
