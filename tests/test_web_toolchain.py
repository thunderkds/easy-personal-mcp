"""HUN-48: release check for the web/ build (scripts/check_static.py) and the real toolchain."""

import json
import subprocess
import sys
import zipfile
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parent.parent
SCRIPT = REPO / "scripts" / "check_static.py"
STATIC_REL = Path("src/easy_personal/dashboard/static")


def run_check(root, *args):
    return subprocess.run(
        [sys.executable, str(SCRIPT), "--root", str(root), *args],
        capture_output=True,
        text=True,
        check=False,
    )


@pytest.fixture
def tree(tmp_path):
    """A minimal fake repo: web sources, lockfile, and a stamped build."""
    web = tmp_path / "web"
    (web / "src").mkdir(parents=True)
    (web / "src" / "main.jsx").write_text("console.log('hi')\n")
    (web / "index.html").write_text('<script type="module" src="/src/main.jsx"></script>\n')
    (web / "package.json").write_text(
        json.dumps({"dependencies": {"react": "19.3.1"}, "devDependencies": {"vite": "8.3.5"}})
    )
    (web / "package-lock.json").write_text(
        json.dumps(
            {
                "lockfileVersion": 3,
                "packages": {
                    "": {"dependencies": {"react": "19.3.1"}},
                    "node_modules/react": {
                        "version": "19.3.1",
                        "resolved": "https://registry.npmjs.org/react/-/react-19.3.1.tgz",
                        "integrity": "sha512-x",
                    },
                },
            }
        )
    )
    static = tmp_path / STATIC_REL
    (static / "assets").mkdir(parents=True)
    (static / "index.html").write_text('<script type="module" src="./assets/app.js"></script>\n')
    (static / "assets" / "app.js").write_text("console.log('built')\n")
    assert run_check(tmp_path, "--write-stamp").returncode == 0
    return tmp_path


def test_fresh_build_passes(tree):
    r = run_check(tree)
    assert r.returncode == 0, r.stdout + r.stderr


def test_missing_static_fails(tree):
    (tree / STATIC_REL / "index.html").unlink()
    r = run_check(tree)
    assert r.returncode == 1
    assert "missing" in (r.stdout + r.stderr).lower()


def test_empty_static_dir_fails(tmp_path):
    (tmp_path / "web" / "src").mkdir(parents=True)
    (tmp_path / STATIC_REL).mkdir(parents=True)
    r = run_check(tmp_path)
    assert r.returncode == 1


def test_stale_build_fails_when_source_changes(tree):
    (tree / "web" / "src" / "main.jsx").write_text("console.log('changed')\n")
    r = run_check(tree)
    assert r.returncode == 1
    assert "stale" in (r.stdout + r.stderr).lower()


def test_stale_build_fails_when_lockfile_changes(tree):
    lock = tree / "web" / "package-lock.json"
    lock.write_text(lock.read_text() + "\n")
    assert run_check(tree).returncode == 1


def test_remote_url_in_built_html_fails(tree):
    (tree / STATIC_REL / "index.html").write_text(
        '<script src="https://cdn.example.com/x.js"></script>\n'
    )
    run_check(tree, "--write-stamp")
    r = run_check(tree)
    assert r.returncode == 1
    assert "cdn.example.com" in (r.stdout + r.stderr)


def test_remote_url_in_built_js_fails(tree):
    (tree / STATIC_REL / "assets" / "app.js").write_text("fetch('http://evil.example/x')\n")
    run_check(tree, "--write-stamp")
    r = run_check(tree)
    assert r.returncode == 1
    assert "evil.example" in (r.stdout + r.stderr)


def test_xml_namespace_and_react_error_urls_are_allowed(tree):
    (tree / STATIC_REL / "assets" / "app.js").write_text(
        "a='http://www.w3.org/2000/svg';b='https://react.dev/errors/'\n"
    )
    run_check(tree, "--write-stamp")
    assert run_check(tree).returncode == 0


def test_sourcemap_reference_fails(tree):
    (tree / STATIC_REL / "assets" / "app.js").write_text("x\n//# sourceMappingURL=app.js.map\n")
    run_check(tree, "--write-stamp")
    r = run_check(tree)
    assert r.returncode == 1
    assert "sourcemap" in (r.stdout + r.stderr).lower()


def test_telemetry_package_in_lockfile_fails(tree):
    lock = tree / "web" / "package-lock.json"
    data = json.loads(lock.read_text())
    data["packages"]["node_modules/posthog-js"] = {
        "version": "1.0.0",
        "resolved": "https://registry.npmjs.org/posthog-js/-/posthog-js-1.0.0.tgz",
        "integrity": "sha512-y",
    }
    lock.write_text(json.dumps(data))
    run_check(tree, "--write-stamp")
    r = run_check(tree)
    assert r.returncode == 1
    assert "posthog-js" in (r.stdout + r.stderr)


def test_unpinned_dependency_fails(tree):
    pkg = tree / "web" / "package.json"
    pkg.write_text(json.dumps({"dependencies": {"react": "^19.3.1"}}))
    run_check(tree, "--write-stamp")
    r = run_check(tree)
    assert r.returncode == 1
    assert "react" in (r.stdout + r.stderr)


def test_wheel_without_static_assets_fails(tree, tmp_path_factory):
    whl = tmp_path_factory.mktemp("whl") / "x-0-py3-none-any.whl"
    with zipfile.ZipFile(whl, "w") as z:
        z.writestr("easy_personal/__init__.py", "")
    r = run_check(tree, "--wheel", str(whl))
    assert r.returncode == 1
    assert "wheel" in (r.stdout + r.stderr).lower()


def test_wheel_with_static_assets_passes(tree, tmp_path_factory):
    whl = tmp_path_factory.mktemp("whl") / "x-0-py3-none-any.whl"
    with zipfile.ZipFile(whl, "w") as z:
        z.writestr("easy_personal/dashboard/static/index.html", "<html></html>")
        z.writestr("easy_personal/dashboard/static/assets/app.js", "x")
    r = run_check(tree, "--wheel", str(whl))
    assert r.returncode == 0, r.stdout + r.stderr


def test_real_repo_build_is_current():
    """Runs against the real tree after `npm --prefix web run build` (see Verification Command)."""
    if not (REPO / STATIC_REL / "index.html").exists():
        pytest.skip("web/ not built in this checkout")
    r = run_check(REPO)
    assert r.returncode == 0, r.stdout + r.stderr


def test_real_wheel_contains_static_assets(tmp_path):
    if not (REPO / STATIC_REL / "index.html").exists():
        pytest.skip("web/ not built in this checkout")
    b = subprocess.run(
        ["uv", "build", "--wheel", "-o", str(tmp_path)],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=False,
    )
    assert b.returncode == 0, b.stderr
    (whl,) = tmp_path.glob("*.whl")
    r = run_check(REPO, "--wheel", str(whl))
    assert r.returncode == 0, r.stdout + r.stderr
