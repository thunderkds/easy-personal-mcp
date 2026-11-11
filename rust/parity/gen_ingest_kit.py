#!/usr/bin/env python3
"""Ingest oracle for the three kit sources (T033 / RS-003): run the real Python `easy-personal ingest` over planted
`kit_trace`, `ui` and `verifier` folders and freeze what it wrote, as goldens.

Same method as `gen_ingest.py` (it reuses its `Case`, guards and writer): each scenario is a list of *steps* (write a
file, replace it, truncate it, symlink, remove, run the ingest ...). The steps are stored in the golden next to the
runs, so the Rust replay test (`rust/crates/core/tests/ingest_kit_replay.rs`) plays the very same steps. After every
`ingest` step the golden holds the exit code, the K keys of the printed JSON, and the tables `kit_trace_events`,
`ui_sessions`, `verifier_scores` and the `kit_trace` rows of `source_files` (ids, machine id and inodes excluded, as T026).

    rust/parity/.venv/bin/python rust/parity/gen_ingest_kit.py           # (re)write rust/parity/ingest/golden-kit/
    rust/parity/.venv/bin/python rust/parity/gen_ingest_kit.py --check   # regenerate in a temp folder and compare: exit 0 equal, 1 different

It only reads the app (imported from src/, never changed). It refuses to run when an env var points at the owner's
real ~/.easy-personal, ~/.claude*, ~/.codex* (the gen_golden.py guard). POSIX only.
"""

import argparse
import hashlib
import json
import os
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
PARITY = Path(__file__).resolve().parent
sys.path.insert(0, str(PARITY))
import gen_golden  # noqa: E402
import gen_ingest as G  # noqa: E402

REPO = G.REPO
GOLDEN = PARITY / "ingest" / "golden-kit"
GENERATOR = "rust/parity/gen_ingest_kit.py"
SOURCES = [REPO / "src/easy_personal/sources" / n for n in ("kit_trace.py", "ui.py", "verifier.py")]
KEEP = ("files", "new_events", "kit_trace", "ui", "verifier")  # `context` is not ported (decided for you)
CANARY = "INVENTED_CANARY"  # planted in `summary`, `target`, `dimensions`: never in a golden, never in a table
EXPAND = ("<WORK>", "<PROJECTS>", "<AGENT_ROOT>", "<HOME>", "<NOWHERE>", "<VERIFIER>", "<UI>", "<CASE>")


class KCase(G.Case):
    """A `Case` whose runs are driven by steps and whose dump is the kit tables."""

    def paths(self) -> dict:
        return {"<WORK>": self.work, "<PROJECTS>": self.projects, "<AGENT_ROOT>": self.agents, "<HOME>": self.home,
                "<NOWHERE>": self.nowhere, "<VERIFIER>": self.verifier, "<UI>": self.root / "ui", "<CASE>": self.root}  # fmt: skip

    def expand(self, text: str) -> str:
        for token, path in self.paths().items():
            text = text.replace(token, str(path))
        return text.replace("%CANARY%", CANARY)

    def env(self, extra=None) -> dict:
        env = super().env()
        for key, value in (extra or {}).items():
            if value is None:
                env.pop(key, None)
            else:
                env[key] = self.expand(value)
        return env

    def body(self, step: dict) -> bytes:
        if "repeat" in step:
            r = step["repeat"]
            return (self.expand(r["prefix"]) + r["unit"] * r["count"] + r.get("close", "") * r["count"] + r["suffix"]).encode()
        return self.expand(step["text"]).encode()

    def play(self, steps: list) -> list:
        for step in steps:
            op = step["op"]
            path = Path(self.expand(step["path"])) if "path" in step else None
            if op in ("write", "append", "rewrite", "replace", "pad", "symlink", "mkfifo"):
                path.parent.mkdir(parents=True, exist_ok=True)
            if op == "mkdir":
                path.mkdir(parents=True, exist_ok=True)
            elif op == "write":
                path.write_bytes(self.body(step))
            elif op == "append":
                with open(path, "ab") as handle:
                    handle.write(self.body(step))
            elif op == "rewrite":  # same inode, shorter or different content
                with open(path, "wb") as handle:
                    handle.write(self.body(step))
            elif op == "replace":  # a new inode: the new file exists before the old one goes
                new = path.with_name(path.name + ".new")
                new.write_bytes(self.body(step))
                os.replace(new, path)
            elif op == "pad":
                with open(path, "wb") as handle:
                    handle.truncate(step["size"])
            elif op == "symlink":
                os.symlink(self.expand(step["target"]), path)
            elif op == "mkfifo":
                os.mkfifo(path)
            elif op == "remove":
                path.unlink()
            elif op == "rmtree":
                shutil.rmtree(path)
            elif op == "transcript":
                lines = [json.dumps({"type": "user", "sessionId": "00000000-0000-4000-8000-000000001030", "timestamp": f"2026-01-10T15:00:0{i}.000Z",
                                     "version": "2.1.400", "cwd": self.expand(cwd), "gitBranch": "main", "isSidechain": False,
                                     "userType": "external", "message": {"role": "user", "content": CANARY}}) for i, cwd in enumerate(step["cwds"])]  # fmt: skip
                target = self.projects / "p" / "session.jsonl"
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text("\n".join(lines) + "\n")
            elif op == "ingest":
                record = self.run(step["label"], ["ingest"], step.get("env"))
                printed = record.get("stdout_json") or {}
                self.kept.append({
                    "label": step["label"], "exit_code": record["exit_code"], "exception": record["exception"],
                    "stdout_json": {k: printed[k] for k in KEEP if k in printed} or None, "tables_after": record["tables_after"],
                })  # fmt: skip
            else:
                raise RuntimeError(f"unknown step {op}")
        return self.kept

    kept: list

    def _dump(self, conn: sqlite3.Connection) -> dict:
        meta = conn.execute("SELECT value FROM meta WHERE key = 'machine_id'").fetchone()[0]
        out = {"machine_id_check": {}}
        queries = {
            "kit_trace_events": ("SELECT * FROM kit_trace_events ORDER BY rowid", ("source_path", "byte_offset", "inode")),
            "ui_sessions": ("SELECT * FROM ui_sessions ORDER BY rowid", ("id",)),
            "verifier_scores": ("SELECT * FROM verifier_scores ORDER BY rowid", ("project_path",)),
            "source_files": ("SELECT * FROM source_files WHERE source = 'kit_trace' ORDER BY rowid", ("path",)),
        }  # fmt: skip
        for table, (sql, order) in queries.items():
            cur = conn.execute(sql)
            columns = [d[0] for d in cur.description]
            rows, same = [], True
            for raw in cur:
                row = dict(zip(columns, raw))
                if "machine_id" in row:
                    same &= row.pop("machine_id") == meta
                    row["inode"] = self.inode_token(row.get("source_path", row.get("path")), row["inode"])
                row.pop("id", None) if table == "kit_trace_events" else None
                rows.append(self.tok_obj(row))
                for value in row.values():
                    if isinstance(value, str) and CANARY in value:
                        raise RuntimeError(f"{table}: a planted canary reached the store")
            if table in ("kit_trace_events", "source_files"):
                out["machine_id_check"][table] = same
            rows.sort(key=lambda r: tuple((r[c] is None, "" if r[c] is None else r[c]) for c in order))
            dropped = ("id", "machine_id") if table == "kit_trace_events" else ("machine_id",)
            out[table] = {"columns": [c for c in columns if c not in dropped], "order": list(order), "rows": rows}
        return out


# -- scenario steps ---------------------------------------------------------------------------------------------


def line(ts="2026-01-10T15:00:00Z", tool="Bash", error=False, **more) -> str:
    return json.dumps({"timestamp": ts, "tool_name": tool, "is_error": error, "summary": "%CANARY%", **more})


def W(path, text="", **more) -> dict:
    return {"op": "write", "path": path, "text": text, **more}


def ING(label, ui=False, **env) -> dict:
    step = {"op": "ingest", "label": label}
    if ui:
        env["EASY_PERSONAL_UI_REPORTS"] = "<UI>"
    if env:
        step["env"] = env
    return step


def TR(*cwds) -> dict:
    return {"op": "transcript", "cwds": list(cwds)}


TRACE = "<WORK>/proj/memory/event-trace"
GOOD = line()


def k01():
    torn = '{"timestamp":"2026-01-10T15:00:09Z","tool_name":"Edit","is_'
    first = "\n".join([
        GOOD, line("2026-01-10T15:00:01Z", "Read", True), "not json", "[1,2]", json.dumps({"timestamp": "2026-01-10T15:00:02Z", "is_error": False}),
        line(tool="bad name"), line(error="yes"), line(error=1), line(ts="2026-01-10T15:00:03"), "", "   \t", "\r",
        line("2026-01-10T15:00:04+05:30", "Edit"), line("2026-01-10T15:00:05.123456789Z", "Bash\n"), line("2026-01-10T15:00:06-12:00", "mcp__easy-ui__x"),
        '{"timestamp":"2026-01-10T15:00:07Z","tool_name":"Bash","is_error":false,"timestamp":"2026-01-10T15:00:08Z"}',
    ]) + "\n" + torn  # fmt: skip
    return "kit_trace: task file with abstained, blank and torn lines, _untagged, odd names, two runs, then the torn line completed", [
        TR("<WORK>/proj"), W(f"{TRACE}/T001.jsonl", first), W(f"{TRACE}/_untagged.jsonl", GOOD + "\n"),
        W(f"{TRACE}/bad name!.jsonl", GOOD + "\n"), W(f"{TRACE}/v1.2.jsonl", GOOD + "\n"), W(f"{TRACE}/.jsonl", GOOD + "\n"),
        W(f"{TRACE}/notes.txt", GOOD + "\n"), W(f"{TRACE}/T9.json", GOOD + "\n"), W(f"{TRACE}/T2.JSONL", GOOD + "\n"),
        ING("first run"), ING("second run, nothing changed (expect new_events 0)"),
        {"op": "append", "path": f"{TRACE}/T001.jsonl", "text": 'error":false}\n' + line("2026-01-10T15:00:10Z", "Write") + "\n"},
        ING("third run after the torn line was completed and one line added"),
    ]  # fmt: skip


def k02():
    three = "\n".join([GOOD, line("2026-01-10T15:00:01Z", "Read"), line("2026-01-10T15:00:02Z", "Edit")]) + "\n"
    two = "\n".join([line("2026-01-10T16:00:00Z", "Grep"), line("2026-01-10T16:00:01Z", "Glob")]) + "\n"
    return "kit_trace: a file replaced by another inode, a file shrunk in place, a file deleted", [
        TR("<WORK>/proj"), W(f"{TRACE}/T001.jsonl", three), ING("first run"),
        {"op": "replace", "path": f"{TRACE}/T001.jsonl", "text": two}, ING("second run: new inode, read from 0 again"),
        {"op": "rewrite", "path": f"{TRACE}/T001.jsonl", "text": line("2026-01-10T17:00:00Z", "Task") + "\n"},
        ING("third run: same inode, smaller than the stored offset (read from 0; rows already stored are kept)"),
        {"op": "remove", "path": f"{TRACE}/T001.jsonl"}, ING("fourth run: the file is gone (its offset row is retired, its events stay)"),
    ]  # fmt: skip


def k03():
    gone, p2 = "<WORK>/gone/memory/event-trace", "<WORK>/proj2/memory/event-trace"
    return "kit_trace: a project that no longer exists, a project path that is a file, a project folder removed between runs", [
        TR("<WORK>/proj", "<WORK>/proj2", "<WORK>/gone", "<WORK>/afile", "<WORK>/nothing"), W("<WORK>/afile", "x"),
        W(f"{TRACE}/T001.jsonl", GOOD + "\n"), W(f"{p2}/T002.jsonl", GOOD + "\n"), W(f"{gone}/T003.jsonl", GOOD + "\n"),
        ING("first run: three projects read"),
        {"op": "rmtree", "path": "<WORK>/gone"}, {"op": "rmtree", "path": "<WORK>/proj2"}, ING("second run: two project folders are gone (offsets retired)"),
    ]  # fmt: skip


def k04():
    real = "<WORK>/outside/real.jsonl"
    return "kit_trace: a symlinked trace file, a broken symlink, a symlinked event-trace folder", [
        TR("<WORK>/proj", "<WORK>/proj2"), W(real, GOOD + "\n" + line("2026-01-10T15:00:01Z", "Read") + "\n"),
        {"op": "symlink", "path": f"{TRACE}/L.jsonl", "target": real}, {"op": "symlink", "path": f"{TRACE}/B.jsonl", "target": "<WORK>/outside/missing.jsonl"},
        W("<WORK>/outside/traces/T2.jsonl", GOOD + "\n"), {"op": "symlink", "path": "<WORK>/proj2/memory/event-trace", "target": "<WORK>/outside/traces"},
        ING("first run"), ING("second run (expect 0 new)"),
    ]  # fmt: skip


def k04c():
    return "kit_trace: a folder named *.jsonl (Python crashes with IsADirectoryError after the files before it; Rust skips it)", [
        TR("<WORK>/proj"), W(f"{TRACE}/A1.jsonl", GOOD + "\n"), {"op": "mkdir", "path": f"{TRACE}/x.jsonl"}, W(f"{TRACE}/z9.jsonl", GOOD + "\n"),
        ING("first run"), ING("second run (still stuck?)"),
    ]  # fmt: skip


def k05():
    ok = '{"id":"s1","status":"passed","startedAt":"2026-01-10T15:00:00Z","endedAt":"2026-01-10T15:00:05Z","target":"%CANARY%","issues":["%CANARY%"],"actions":[{"x":"%CANARY%"},2,3]}'

    def s(name, text, **more):
        return W(f"<UI>/{name}", text, **more)

    deep = {"prefix": '{"id":"deep","status":"x","startedAt":"2026-01-10T15:00:00Z","actions":', "unit": "[", "close": "]", "count": 60000, "suffix": "}"}
    label200 = "aaaaaaaaa_" * 20
    return "ui: every shape of session file (ok, bad JSON, wrong types, duplicate ids, BOM, oversize, deep nesting, symlinks, a folder)", [
        s("session-a.json", ok),
        s("session-b.json", '{"id":"s2","status":"failed","startedAt":"2026-01-10T16:00:00Z","actions":[]}'),
        s("session-c.json", '{"id":"s3","status":"passed","startedAt":"2026-01-10T17:00:00Z","endedAt":null,"actions":[1]}'),
        s("session-d.json", '{"id":"s4","status":"passed","startedAt":"2026-01-10T17:00:00Z","endedAt":"garbage","actions":[]}'),
        s("session-e.json", "{"), s("session-f.json", "[1]"),
        s("session-g.json", '{"id":"s5","status":"passed","startedAt":"2026-01-10T17:00:00Z"}'),
        s("session-h.json", '{"id":"s6","status":"passed","startedAt":"2026-01-10T17:00:00","actions":[]}'),
        s("session-i.json", '{"id":"bad id","status":"passed","startedAt":"2026-01-10T17:00:00Z","actions":[]}'),
        s("session-j.json", '{"id":"s7","status":5,"startedAt":"2026-01-10T17:00:00Z","actions":[]}'),
        s("session-k.json", '{"id":"s1","status":"failed","startedAt":"2026-01-10T18:00:00Z","actions":[1,2]}'),
        s("session-l.json", '{"id":"s8","status":"passed","startedAt":"2026-01-10T17:00:00Z","actions":{}}'),
        s("session-.json", '{"id":"empty-star","status":"passed","startedAt":"2026-01-10T17:00:00Z","actions":[]}'),
        s("session-m.json", '﻿{"id":"bom","status":"passed","startedAt":"2026-01-10T17:00:00Z","actions":[]}'),
        {"op": "pad", "path": "<UI>/session-o.json", "size": 20000001}, s("session-p.json", "", repeat=deep),
        {"op": "mkdir", "path": "<UI>/session-q.json"},
        W("<WORK>/outside/real-session.json", '{"id":"linked","status":"passed","startedAt":"2026-01-10T17:00:00Z","actions":[]}'),
        {"op": "symlink", "path": "<UI>/session-r.json", "target": "<WORK>/outside/real-session.json"},
        {"op": "symlink", "path": "<UI>/session-s.json", "target": "<WORK>/outside/missing.json"},
        s("session-t.json", '{"id":"frac","status":"passed","startedAt":"2026-01-10T15:00:00.123456789+05:30","actions":[]}'),
        s("session-u.json", f'{{"id":"{label200}","status":"passed","startedAt":"2026-01-10 15:00:00Z","actions":[]}}'),
        s("session-v.json", f'{{"id":"{label200}x","status":"passed","startedAt":"2026-01-10T15:00:00Z","actions":[]}}'),
        s("notes.json", ok), s("session-x.txt", ok), s("Session-z.json", ok), s("session-w.json.bak", ok),
        ING("first run", ui=True), ING("second run (the same files)", ui=True),
    ]  # fmt: skip


def k06():
    def ses(sid, status):
        return f'{{"id":"{sid}","status":"{status}","startedAt":"2026-01-10T15:00:00Z","actions":[1]}}'

    cfg = "# owner settings\nthresholds:\n  tool_calls: 5\nui:\n  reports_dir: ~/ui-cfg\nverifier:\n  reports_dir: <CASE>/v\n"
    return "ui: unset and missing folders wipe the table, an empty folder does not, replace without delete, env beats config, ~ and flat-file quirks", [
        W("<UI>/session-a.json", ses("a", "passed")), ING("env folder set: one session", ui=True),
        ING("env unset: wipes ui_sessions (status no data)"), ING("env folder set again: the row is back", ui=True),
        {"op": "mkdir", "path": "<CASE>/ui-empty"}, ING("existing empty folder: no wipe", EASY_PERSONAL_UI_REPORTS="<CASE>/ui-empty"),
        W("<UI>/session-a.json", ses("a", "failed")), W("<UI>/session-b.json", ses("b", "passed")), ING("a changed, b added: replace only", ui=True),
        {"op": "remove", "path": "<UI>/session-a.json"}, ING("a's file removed: its row stays (no delete)", ui=True),
        ING("missing folder: wipes", EASY_PERSONAL_UI_REPORTS="<CASE>/ui-missing"),
        W("<CASE>/cfg-sessions/session-c.json", ses("c", "passed")), W("<HOME>/config.yaml", cfg.replace("~/ui-cfg", "<CASE>/cfg-sessions")),
        ING("env unset, config.yaml ui: reports_dir"), ING("env empty string: config is used", EASY_PERSONAL_UI_REPORTS=""),
        ING("env set beats config", ui=True),
        W("<NOWHERE>/ui-cfg/session-d.json", ses("d", "passed")), W("<HOME>/config.yaml", cfg), ING("config reports_dir with ~ (HOME is the nowhere folder)"),
        W("<HOME>/config.yaml", "ui:\n\treports_dir: <UI>\n"), ING("tab-indented value is not inside the section: wipes"),
        W("<HOME>/config.yaml", "ui: # note\n  reports_dir: <UI>  \n  other: 1\n"), ING("`ui:` with a comment, trailing spaces in the value"),
        W("<HOME>/config.yaml", "ui :\n  reports_dir: <UI>\n"), ING("`ui :` is another section: wipes"),
    ]  # fmt: skip


def report(value=86.5, abstained=False, reason=None, version=1, scope="changes", generated="2026-01-10T15:00:00Z", **more) -> str:
    return json.dumps({"schema_version": version, "scope": scope, "generated_at": generated,
                       "overall": {"abstained": abstained, "value": value, "reason_code": reason}, "dimensions": ["%CANARY%"], **more})  # fmt: skip


def stamp(day, n=1) -> str:
    return f"evidence-report-202601{day:02d}T000000-{n}Z.json"


def k07():
    w = "<WORK>"
    steps = [TR(*[f"{w}/{n}" for n in ("pa", "pb", "pc", "pd", "pe", "g1/same", "g2/same", "ph", "pi", "pj")])]
    for name in ("pa", "pb", "pc", "pd", "pe", "g1/same", "g2/same", "ph", "pi", "pj"):
        steps.append({"op": "mkdir", "path": f"{w}/{name}"})
    steps += [
        W(f"{w}/pa/reports/{stamp(1)}", report(70)), W(f"{w}/pa/reports/{stamp(2)}", report(86.5)), W(f"{w}/pa/reports/evidence-report-latest.json", report(1)),
        W(f"{w}/pa/reports/{stamp(2, 2)}.bak", report(2)),
        W(f"{w}/pb/reports/{stamp(1)}", report(60)), W(f"<VERIFIER>/pb/{stamp(5)}", report(version=2)),
        W(f"{w}/pc/reports/{stamp(3)}", report(None, True, "no_evidence")),
    ]  # fmt: skip
    bad = [report(101), report(-1), report("50"), report(True), report(5, True), report(None, False), report(5, reason="bad reason"), report(scope="bad scope"),
           report(generated="2026-01-10T00:00:00"), report(version="1"), report(version=True), report(version=1.0), "[]", "{", report(float("nan"))]  # fmt: skip
    for i, text in enumerate(bad):
        steps.append(W(f"{w}/pd/reports/{stamp(1 + i // 3, i)}", text))
    steps += [
        W(f"{w}/pe/reports/{stamp(1)}", report(55)), {"op": "symlink", "path": f"{w}/pe/reports/{stamp(2)}", "target": f"{w}/pa/reports/{stamp(2)}"},
        {"op": "pad", "path": f"{w}/pe/reports/{stamp(3)}", "size": 1000001}, {"op": "mkfifo", "path": f"{w}/pe/reports/{stamp(4)}"},
        {"op": "mkdir", "path": f"{w}/pe/reports/{stamp(5)}"},
        W("<VERIFIER>/same/" + stamp(9), report(99)), W(f"{w}/g1/same/reports/{stamp(1)}", report(11)),
        W(f"{w}/outside/reports/{stamp(1)}", report(33)), {"op": "symlink", "path": f"{w}/ph/reports", "target": f"{w}/outside/reports"},
        W(f"{w}/outside/pi/{stamp(1)}", report(44)), {"op": "symlink", "path": "<VERIFIER>/pi", "target": f"{w}/outside/pi"},
        W(f"<VERIFIER>/pj/{stamp(1)}", report(100)), W(f"<VERIFIER>/pj/{stamp(2)}", report(0)),
        W(f"<VERIFIER>/pj/{stamp(3)}", report(0).replace('"value": 0,', '"value": 1e2,')),
        ING("first run: selection by name stamp, unknown schema, abstained, invalid, symlink, oversize, fifo, folder, ambiguous name, symlinked folders"),
        ING("second run (same files, the table is replaced)"),
        {"op": "rmtree", "path": "<VERIFIER>"}, ING("container folder gone: only <project>/reports is read"),
        {"op": "rmtree", "path": f"{w}/pa/reports"}, {"op": "rmtree", "path": f"{w}/pb/reports"}, {"op": "rmtree", "path": f"{w}/pc/reports"},
        {"op": "rmtree", "path": f"{w}/pe/reports"}, {"op": "rmtree", "path": f"{w}/g1/same/reports"}, {"op": "rmtree", "path": f"{w}/pd/reports"},
        ING("no readable report anywhere: the table is emptied (status no data)"),
    ]  # fmt: skip
    return "verifier: newest report by name stamp per project, every skip rule, replace-all, the container folder", steps


def k08():
    p = "<WORK>/proj"
    cfg = "verifier:\n  reports_dir: <CASE>/vc\n"
    return "verifier: container from the default folder under HOME, from config.yaml, from the env (env > config > default)", [
        TR(p), {"op": "mkdir", "path": p},
        W(f"<NOWHERE>/.easy-verifier-reports/proj/{stamp(1)}", report(10)), ING("default container under HOME", EASY_PERSONAL_VERIFIER_REPORTS=None),
        W(f"<CASE>/vc/proj/{stamp(1)}", report(20)), W("<HOME>/config.yaml", cfg), ING("config.yaml verifier: reports_dir", EASY_PERSONAL_VERIFIER_REPORTS=None),
        W(f"<VERIFIER>/proj/{stamp(1)}", report(30)), ING("env beats config"),
        W("<HOME>/config.yaml", "verifier:\n  reports_dir: ~/home-reports\n"), W(f"<NOWHERE>/home-reports/proj/{stamp(1)}", report(40)),
        ING("config with ~", EASY_PERSONAL_VERIFIER_REPORTS=None),
        W(f"<VERIFIER>/proj/{stamp(2)}", report(version=3)), ING("a newer unknown-schema file turns the project abstained"),
    ]  # fmt: skip


def k09():
    return "an empty store: no events, no projects, nothing to read", [ING("first run", ui=False)]


SCENARIOS = [("K01_kit_trace_lines", k01), ("K02_kit_trace_inode_shrunk_deleted", k02), ("K03_kit_trace_projects_gone", k03),
             ("K04_kit_trace_symlinks", k04), ("K04c_crash_directory_named_jsonl", k04c), ("K05_ui_files", k05), ("K06_ui_folder_rules", k06),
             ("K07_verifier_reports", k07), ("K08_verifier_container", k08), ("K09_empty_store", k09)]  # fmt: skip


# -- writing ---------------------------------------------------------------------------------------------------


def generate(out: Path, run_root: Path) -> list:
    import easy_personal

    if Path(easy_personal.__file__).resolve().parent != REPO / "src" / "easy_personal":
        raise RuntimeError(f"easy_personal imports from {easy_personal.__file__}, not src/")
    out.mkdir(parents=True)
    texts, scenarios = {}, []
    for scenario_id, function in SCENARIOS:
        case = KCase(run_root, scenario_id)
        case.kept = []
        what, steps = function()
        runs = case.play(steps)
        texts[f"{scenario_id}.json"] = G.render({"scenario": scenario_id, "what": what, "steps": steps, "runs": runs})
        scenarios.append({"id": scenario_id, "file": f"{scenario_id}.json", "what": what, "exit_codes": [r["exit_code"] for r in runs]})
        if any(str(run_root) in text for text in texts.values()):
            raise RuntimeError(f"{scenario_id}: a temp path survived the tokenising")
    manifest = {
        "generator": GENERATOR,
        "note": "Python `easy-personal ingest` over planted kit_trace, ui and verifier folders (T033). Per scenario file: the steps (the fixtures: "
        "the Rust replay plays the same ones), then one run per `ingest` step with the exit code, the K keys of the printed JSON and the tables.",
        "versions": G.versions(),
        "sources_sha256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in SOURCES},
        "work_path_length": G.WORK_LEN,
        "tokens": {t: "a folder of the scenario's case (see KCase.paths)" for t in EXPAND} | {"%CANARY%": "a planted text: never stored"},
        "scenarios": scenarios,
        "files": {name: hashlib.sha256(text.encode("utf-8")).hexdigest() for name, text in sorted(texts.items())},
    }
    texts["manifest.json"] = json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    for name, text in sorted(texts.items()):
        data = text.encode("utf-8")
        G.leak_scan(name, data, golden=True)
        (out / name).write_bytes(data)
    return sorted(texts)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--check", action="store_true", help="regenerate in a temp folder, compare with the committed goldens")
    args = parser.parse_args()
    gen_golden.refuse_real_paths(GOLDEN)
    gen_golden.refuse_fixture_folder()
    base = Path(tempfile.gettempdir()).resolve()
    G.refuse("TMPDIR", base)
    if str(base).startswith("/mnt/"):
        sys.exit("refused: the temp folder is on /mnt (WSL drive: flock may not hold); set TMPDIR=/tmp")
    if GOLDEN.exists() and not args.check:
        try:
            ours = json.loads((GOLDEN / "manifest.json").read_text())["generator"] == GENERATOR
        except (OSError, ValueError, LookupError, TypeError):
            ours = False
        if not ours:
            sys.exit(f"refused: {GOLDEN} was not made by this generator; nothing was deleted")
    if subprocess.run(["git", "-C", str(base), "rev-parse", "--show-toplevel"], capture_output=True, timeout=30,
                      env={"PATH": os.environ.get("PATH", ""), "GIT_CONFIG_GLOBAL": "/dev/null"}).returncode == 0:  # fmt: skip
        sys.exit("refused: the temp folder is inside a git repo; set TMPDIR")
    with tempfile.TemporaryDirectory(prefix="ingest-kit-oracle-", dir=base) as tmp:
        run_root = Path(tmp).resolve()
        G.isolate(run_root)
        new = run_root / "golden"
        names = generate(new, run_root)
        if args.check:
            diffs = G.compare(new, GOLDEN)
            for text in diffs:
                print(text)
            print(f"--check: {len(names)} files, {len(diffs)} different")
            sys.exit(1 if diffs else 0)
        GOLDEN.mkdir(parents=True, exist_ok=True)
        old = set(json.loads((GOLDEN / "manifest.json").read_text())["files"]) | {"manifest.json"} if (GOLDEN / "manifest.json").is_file() else set()
        for name in sorted(old - set(names)):
            (GOLDEN / name).unlink(missing_ok=True)
        for name in names:
            shutil.copyfile(new / name, GOLDEN / name)
    print(f"{len(names)} golden files in {GOLDEN.relative_to(REPO)} ({len(SCENARIOS)} scenarios)")


if __name__ == "__main__":
    main()
