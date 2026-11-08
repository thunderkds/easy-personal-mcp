"""HUN-242 (FR-090): `GET /api/kit/report-files` and `GET /api/kit/report-file`, behind the token and Host check."""

from urllib.parse import quote

import pytest
from starlette.testclient import TestClient

from easy_personal.core import config, store, workspaces
from easy_personal.surfaces import api

TOKEN, PORT = "tok-242", 5542
HOST = f"127.0.0.1:{PORT}"
AUTH = {api.TOKEN_HEADER: TOKEN}
LIST, READ = "/api/kit/report-files", "/api/kit/report-file"
BASE = {"workspace": "acme", "repo_no": "0", "folder": "reports"}
NOT_FOUND = {"error": "not found"}


@pytest.fixture
def world(tmp_path, monkeypatch):
    root = tmp_path.resolve()
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(root / "home"))
    app = root / "app"
    (app / "reports" / "sub").mkdir(parents=True)
    (app / "reports" / "a.html").write_text("<h1>hi</h1>")
    (app / "reports" / "sub" / "b c#d.md").write_text("# b")
    (app / "reports" / "a.svg").write_text("<svg/>")
    with store.opened(config.store_path()) as conn:
        workspaces.add(conn, "acme", repos=[str(app)])
        conn.commit()
    client = TestClient(api.create_app(TOKEN, PORT, root / "absent"), base_url=f"http://{HOST}")
    client.root = root
    return client


def test_reports_route_carries_repo_no(world):
    (folder,) = world.get("/api/kit/reports", headers=AUTH).json()["workspaces"][0]["folders"]
    assert folder["repo_no"] == 0


def test_list_route_answers_files(world):
    res = world.get(LIST, params=BASE, headers=AUTH)
    assert res.status_code == 200 and set(res.json()) == {"files", "truncated"}
    assert {f["path"]: f["openable"] for f in res.json()["files"]} == {"a.html": True, "sub/b c#d.md": True, "a.svg": False}


def test_read_route_headers_and_bytes(world):
    res = world.get(READ, params={**BASE, "path": "a.html"}, headers=AUTH)
    assert res.status_code == 200 and res.content == b"<h1>hi</h1>"
    assert res.headers["content-type"] == "text/html; charset=utf-8"
    assert res.headers["x-content-type-options"] == "nosniff"
    assert res.headers["cache-control"] == "no-store"
    assert res.headers["content-disposition"] == "inline"
    assert res.headers["content-security-policy"] == api.CSP


def test_odd_file_names_round_trip_through_the_query_string(world):
    res = world.get(READ + "?workspace=acme&repo_no=0&folder=reports&path=" + quote("sub/b c#d.md"), headers=AUTH)
    assert res.status_code == 200 and res.content == b"# b"


@pytest.mark.parametrize(
    "query",
    [
        {**BASE, "path": "../x"}, {**BASE, "path": "a.svg"}, {**BASE, "path": "nope.txt"}, {**BASE},
        {**BASE, "path": "a.html", "repo_no": "abc"}, {**BASE, "path": "a.html", "repo_no": "-1"},
        {**BASE, "path": "a.html", "repo_no": "99"}, {**BASE, "path": "a.html", "repo_no": " 0"},
        {**BASE, "path": "a.html", "repo_no": "٠"}, {**BASE, "path": "a.html", "folder": "src"},
        {**BASE, "path": "a.html", "workspace": "nope"}, {"path": "a.html"},
    ],
)  # fmt: skip
def test_read_misses_are_a_fixed_404(world, query):
    res = world.get(READ, params=query, headers=AUTH)
    assert (res.status_code, res.json()) == (404, NOT_FOUND)


@pytest.mark.parametrize("query", [{**BASE, "repo_no": "abc"}, {**BASE, "folder": "src"}, {**BASE, "workspace": "nope"}, {}])
def test_list_misses_are_a_fixed_404(world, query):
    res = world.get(LIST, params=query, headers=AUTH)
    assert (res.status_code, res.json()) == (404, NOT_FOUND)


def test_too_large_is_413(world):
    big = world.root / "app" / "reports" / "big.txt"
    big.write_bytes(b"")
    big.write_bytes(b"\0" * 10)
    import os

    os.truncate(big, 10 * 1024 * 1024 + 1)
    res = world.get(READ, params={**BASE, "path": "big.txt"}, headers=AUTH)
    assert (res.status_code, res.json()) == (413, {"error": "too large"})


@pytest.mark.parametrize("url", [LIST, READ])
def test_routes_refuse_no_token_wrong_token_wrong_host_and_foreign_origin(world, url):
    q = {**BASE, "path": "a.html"}
    assert world.get(url, params=q).status_code == 403
    assert world.get(url, params=q, headers={api.TOKEN_HEADER: "nope"}).status_code == 403
    assert world.get(url, params=q, headers={**AUTH, "Host": "evil.example"}).status_code == 403
    assert world.get(url, params=q, headers={**AUTH, "Origin": "http://evil.example"}).status_code == 403


@pytest.mark.parametrize("url", [LIST, READ])
def test_the_token_is_not_accepted_from_the_query_string(world, url):
    assert world.get(url, params={**BASE, "path": "a.html", "token": TOKEN}).status_code == 403


@pytest.mark.parametrize("url", [LIST, READ])
def test_they_are_read_only_routes(world, url):
    for method in ("POST", "PUT", "PATCH", "DELETE"):
        assert world.request(method, url, params={**BASE, "path": "a.html"}, headers=AUTH, content="{}").status_code == 405


def test_an_unreadable_mtime_does_not_make_either_route_answer_500(world):
    """A file with an mtime `datetime` cannot represent must not break the HTTP answer (FR-089, FR-090)."""
    import os

    bad = world.root / "app" / "reports" / "bad.txt"
    bad.write_text("x")
    try:
        os.utime(bad, (10**17, 10**17))
    except (OSError, OverflowError):
        pytest.skip("the filesystem will not store an out-of-range mtime")
    if os.lstat(bad).st_mtime < 10**16:
        pytest.skip("the filesystem clamped the out-of-range mtime")

    folders = world.get("/api/kit/reports", headers=AUTH)
    listing = world.get(LIST, params=BASE, headers=AUTH)

    assert folders.status_code == 200, folders.text
    assert listing.status_code == 200, listing.text
    by_path = {f["path"]: f for f in listing.json()["files"]}
    assert by_path["bad.txt"]["mtime_utc"] is None
    assert by_path["a.html"]["mtime_utc"] is not None
    # the other file still reads normally
    assert world.get(READ, params={**BASE, "path": "a.html"}, headers=AUTH).status_code == 200
