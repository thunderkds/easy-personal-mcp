"""`GET /api/kit/report-files` and `/report-file` show only what FR-090 allows (HUN-242, NFR-002).

The list carries relative paths inside the opened report folder. A file NAME or CONTENT outside the opened
path, and the absolute path, never show in any answer, error body or log. A file CONTENT shows only in the
read answer of its own file.
"""

from urllib.parse import quote

from starlette.testclient import TestClient

from easy_personal.core import config, store, workspaces
from easy_personal.surfaces import api
from leak import harness
from leak.canaries import Canaries

TOKEN, PORT = "leak-242", 5542
HEADERS = {api.TOKEN_HEADER: TOKEN}
Q = "?workspace=acme&repo_no=0&folder=reports"


def test_names_contents_and_absolute_paths_stay_out_of_every_answer_and_log(tmp_path, monkeypatch):
    canaries = Canaries.new()
    root = tmp_path.resolve()
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(root / "home"))
    names = [v for v in canaries.values.values() if v.isprintable() and not set(v) & set("/\0")]
    other_name, other_dir, secret_name = names[0], names[1], names[2]
    content = "\n".join(canaries.values.values())
    repo = root / "deep" / "tree" / "planted-repo"
    reports = repo / "reports"
    (reports / "opened.txt").parent.mkdir(parents=True)
    (reports / "opened.txt").write_text(content)  # the one file that is read
    (reports / f"{other_name}.txt").write_text(content)  # another report file: name and content
    (repo / "src").mkdir()
    (repo / "src" / f"{secret_name}.txt").write_text(content)  # outside any report folder
    (repo / "reports2-reports").mkdir()
    (repo / "reports2-reports" / f"{other_dir}.txt").write_text(content)  # another report folder
    with store.opened(config.store_path()) as conn:
        workspaces.add(conn, "acme", repos=[str(repo)])
        conn.commit()
    client = TestClient(api.create_app(TOKEN, PORT), base_url=f"http://127.0.0.1:{PORT}")
    calls = [
        "/api/kit/report-files" + Q,
        "/api/kit/report-file" + Q + "&path=" + quote(f"{other_name}.txt"),  # the file of the list entry
        "/api/kit/report-file" + Q + "&path=..%2Fsrc%2F" + quote(f"{secret_name}.txt"),
        "/api/kit/report-file" + Q + "&path=" + quote(f"{secret_name}.txt"),
        "/api/kit/report-file" + Q + "&path=%2Fetc%2Fpasswd",
        "/api/kit/report-file" + Q + "&path=nope.svg",
        "/api/kit/report-file?workspace=acme&repo_no=0&folder=" + quote(secret_name),
        "/api/kit/report-files?workspace=" + quote(other_dir) + "&repo_no=9&folder=" + quote(secret_name),
    ]
    with harness.LogCapture() as logs:
        results = [(c, client.get(c, headers=HEADERS)) for c in calls]
        log_text = "\n".join(l for l in logs.lines if not l.startswith("httpx "))  # the test client logs its own request URL; the server logs nothing
    listing, read, *errors = (res for _, res in results)
    assert listing.status_code == 200 and read.status_code == 200
    assert read.text == content  # the content shows in the read answer of its own file
    assert "opened.txt" in listing.text and f"{other_name}.txt" in listing.text  # relative paths inside the opened folder
    assert all(res.status_code == 404 for res in errors)
    inside = {"opened.txt", f"{other_name}.txt"}
    for blob in (f"{listing.status_code} {dict(listing.headers)} {listing.text}", *(f"{r.status_code} {dict(r.headers)} {r.text}" for r in errors)):
        assert secret_name not in blob and other_dir not in blob and str(root) not in blob and "deep/tree" not in blob
        assert content not in blob
        assert blob.startswith("200") or not any(v in blob for v in canaries.values.values())
        assert not any(name in blob for name in inside) or blob.startswith("200")  # only the list names the files
    for value in canaries.values.values():
        assert value not in log_text
    assert str(root) not in log_text and "deep/tree" not in log_text and "opened.txt" not in log_text
