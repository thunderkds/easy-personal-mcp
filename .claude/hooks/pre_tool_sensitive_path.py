#!/usr/bin/env python3
"""
PreToolUse hook (T146) — fires before every Edit, Write and Bash tool call.

Asks the human first when the call touches a path listed in
`.claude/sensitive-paths.txt` (`.env*`, secrets, production configs, infra),
unless the active task's guide predicted that exact path under
`## Files to Change (Predicted)`.

Decisions, and the direction every uncertainty resolves:

* Sensitive and not predicted -> `ask`, the reason naming the path and pattern.
* Sensitive and predicted, or not sensitive -> no output, exit 0: "no decision,
  normal permission flow applies". Deliberately NOT `allow` — `allow` skips the
  permission prompt for the *whole* call, which grants more than this hook's
  concern (see tasks/TASK_REVIEW_T146.md, "Assumption check").
* Malformed stdin, an unreadable pattern file, or a helper module that fails to
  import -> `ask`. Fail toward the human, never a silent allow (AC5).

Board mode `online` (T157, NFR-005): guides are pulled from the board, whose text anyone
with workspace access can write, so no guide's prediction exempts a sensitive path — it
always asks. An unusable `.claude/board.json` counts as online here (the safe side).

Online approve-once (T314): the trust source is the user's own yes, never board text.
Approvals apply to Edit and Write only: their `file_path` is exact. Every Bash call
that touches a sensitive path asks, as before, approved or not. When this hook asks
about an Edit or Write in online mode with an active task, it writes
`.claude/state/pending/<tool_use_id>.json` = `{"task_id": T, "keys": [...]}`. If the
user says yes, the call runs and the PostToolUse recorder
`post_tool_sensitive_approval.py` moves exactly those keys under T into
`.claude/state/sensitive-approvals.json` (gitignored), but only when the event's
`permission_mode` is `default` or `auto` (the modes where an `ask` shows the prompt).
Keys are files only, never a directory: one key per file, root-relative under the
project root, else its absolute realpath. A later Edit or Write in T of an approved
file gets no output (no decision, never `allow`). When the key differs from the path
as written (a symlink), the ask names both: `config/prod.json → .env`. No active task
-> no approval, every call asks. Always sensitive and never approvable (hard-coded
below, not in the pattern list): `.claude/state` and everything under it, any file
named `sensitive-approvals.json*`, and `.claude` or `.claude/state` named in a Bash
call that changes directory or uses `-C`. A symlinked or bad approval file counts as
empty. Local mode neither reads nor writes any of it. To withdraw every approval,
delete the file.

Bash: the command's heredoc bodies and its *data* quoted spans are removed with
`lib/shell_data.py` first — the only sanctioned way to separate command text from
data (T099's three fail-opens came from fresh regexes). Quoted spans T099 classes
as executable are kept, so `bash -c "cat > .env"` still asks. What is left is
split into tokens and each token is checked as a path.

Recorded limitations (cut list, TASK_GUIDE_T146):
* Paths built from variables (`cat "$F"`) are not resolved.
* A Bash command that only *reads* a sensitive path (`grep KEY .env`) asks too —
  reads and writes are not cheaply distinguishable in a command string.
"""
import glob
import itertools
import json
import os
import re
import sys

HOOKS_DIR = os.path.dirname(os.path.abspath(__file__))
PATTERNS_RELPATH = os.path.join(".claude", "sensitive-paths.txt")
PREDICTED_HEADING = "Files to Change (Predicted)"
# What is passed to read_guide_section: its slicer puts `\b` after the heading,
# which can never match after a closing `)`. The slicer accepts a trailing
# qualifier, so the bare prefix resolves `## Files to Change (Predicted)`.
PREDICTED_LOOKUP = "Files to Change"
TAG = "[hook:sensitive-path]"
# T314: the online approval state. `.claude/state` and everything under it is always
# sensitive, protected here rather than by a line in `.claude/sensitive-paths.txt`, so
# a project's list edit cannot drop it.
STATE_RELDIR = ".claude/state"
STATE_PATTERN = STATE_RELDIR + "/**"
APPROVALS_RELPATH = STATE_RELDIR + "/sensitive-approvals.json"
PENDING_RELDIR = STATE_RELDIR + "/pending"
# S2: the approval file's name is sensitive in any directory (a `cd .claude` call can
# write it under a relative path no root-anchored pattern sees).
APPROVALS_NAME_PATTERNS = ("sensitive-approvals.json", "sensitive-approvals.json.*")
PROTECTED_PATTERNS = (STATE_PATTERN,) + APPROVALS_NAME_PATTERNS
# S2: a Bash token naming `.claude` or `.claude/state` asks when the call changes
# directory or uses `-C`, because then a relative path may land in the state folder.
DIR_CHANGE_TOKENS = {"cd", "pushd", "-C", "--directory"}
STATE_PARENTS = (".claude", STATE_RELDIR)
APPROVABLE_TOOLS = ("Edit", "Write")
TOOL_USE_ID_PATTERN = re.compile(r"[A-Za-z0-9_-]{1,200}\Z")

# Guarded like pre_bash_block_unsafe_merge.py: an unguarded ImportError exits 1
# with empty stdout, which the harness treats as a non-blocking error and lets
# the call through — the silent allow AC5 forbids. The failure is kept and
# turned into an `ask` in main().
sys.path.insert(0, os.path.join(HOOKS_DIR, "lib"))
try:
    from project_root import exit_outside_kit_project  # noqa: E402
    from project_root import project_root as project_dir  # noqa: E402
    exit_outside_kit_project()  # T232: first, so a broken lib stays silent in a non-kit project
    from guide_sections import read_guide_section  # noqa: E402
    from shell_data import strip_heredoc_bodies, strip_quoted_spans  # noqa: E402
    from task_context import resolve_task_id  # noqa: E402
    from board.moments import board_mode  # noqa: E402
    IMPORT_ERROR = None
except Exception as exc:  # pragma: no cover - exercised via subprocess test
    IMPORT_ERROR = exc

# Where a shell token ends. Quote characters are separators too, because a
# quoted span T099 keeps as code (`bash -c "cat > .env"`) arrives with its quotes.
# `:` too (T314): `git show :.env` and `HEAD:.env` read the file; splitting only adds asks.
TOKEN_SPLIT_PATTERN = re.compile(r"[\s;&|<>()`'\"=:]+")
GLOB_CHARS = re.compile(r"[*?\[]")
MAX_GLOB_MATCHES = 200
BACKTICK_SPAN_PATTERN = re.compile(r"`([^`]+)`")
# A quoted word right after an output redirect (`>`, `>>`, `>|`, `&>`, `2>`).
# strip_quoted_spans treats every quoted span behind `echo`/`printf` as data,
# but a redirect target is where the output is written, not data.
QUOTED_REDIRECT_TARGET_PATTERN = re.compile(r"""(?:&|\d*)>[>|]?\s*(?:"([^"]*)"|'([^']*)')""")


# --- Pattern file ------------------------------------------------------------

def pattern_file_path(root):
    """The project's list; the plugin's own copy only when the project has none (T233).

    A hook run from a plugin cache sits at `<plugin>/.claude/hooks`, so the copy shipped with the
    plugin is `<plugin>/.claude/sensitive-paths.txt`. In the `setup.sh` install the two paths are
    the same file, so nothing changes there. A project file that exists but cannot be read is
    still returned, so the caller asks; it never falls back.
    """
    project_file = os.path.join(root, PATTERNS_RELPATH)
    if os.path.lexists(project_file):
        return project_file
    plugin_file = os.path.join(os.path.dirname(HOOKS_DIR), "sensitive-paths.txt")
    return plugin_file if os.path.lexists(plugin_file) else project_file


def expand_braces(pattern):
    """`prod*.{yml,json}` -> [`prod*.yml`, `prod*.json`]. Nested groups recurse."""
    match = re.search(r"\{([^{}]*)\}", pattern)
    if not match:
        return [pattern]
    head, tail = pattern[:match.start()], pattern[match.end():]
    expanded = []
    for option in match.group(1).split(","):
        expanded.extend(expand_braces(head + option + tail))
    return expanded


def glob_to_regex(glob_pattern):
    """Translate one brace-free glob: `*` stays in a directory, `**` crosses them."""
    out = []
    i = 0
    while i < len(glob_pattern):
        if glob_pattern.startswith("**/", i):
            out.append("(?:.*/)?")
            i += 3
        elif glob_pattern.startswith("**", i):
            out.append(".*")
            i += 2
        elif glob_pattern[i] == "*":
            out.append("[^/]*")
            i += 1
        elif glob_pattern[i] == "?":
            out.append("[^/]")
            i += 1
        else:
            out.append(re.escape(glob_pattern[i]))
            i += 1
    return re.compile("".join(out) + r"\Z")


def parse_patterns(text):
    """Return `[(raw_line, negated, has_slash, [compiled regex, ...]), ...]`.

    Windows separators are normalised to `/`. A pattern with no `/` matches a
    file name in any directory; one with a `/` matches from the project root.
    """
    patterns = []
    for line in text.splitlines():
        raw = line.strip()
        if not raw or raw.startswith("#"):
            continue
        negated = raw.startswith("!")
        body = raw[1:] if negated else raw
        body = body.replace("\\", "/").lstrip("/")
        if not body:
            continue
        regexes = [glob_to_regex(g) for g in expand_braces(body)]
        patterns.append((raw, negated, "/" in body, regexes))
    return patterns


def load_patterns(path):
    """Read and parse the pattern file. Raises OSError when it is unreadable."""
    with open(path, encoding="utf-8") as f:
        return parse_patterns(f.read())


# --- Path matching -----------------------------------------------------------

def _within(path, root):
    try:
        rel = os.path.relpath(path, root)
    except ValueError:
        return None
    if rel == "." or rel == ".." or rel.startswith(".." + os.sep):
        return None
    return rel.replace(os.sep, "/")


def path_forms(path, cwd, roots):
    """Every form a path is checked in, as `(anchored, path_string)` pairs.

    Anchored: relative to a root it lies under (the project dir or the call's
    cwd — a Stage 3 agent's cwd is its own worktree). Both the path as given and
    its resolved form are checked, so a symlink cannot route around a pattern.
    A path under no root is *floating*: a `/` pattern may match from any of its
    directories, which over-asks rather than guesses an anchor.
    """
    path = path.replace("\\", "/")
    absolute = os.path.normpath(os.path.join(cwd, os.path.expanduser(path)))
    forms = []
    for candidate in {absolute, os.path.realpath(absolute)}:
        anchored = [rel for rel in (_within(candidate, r) for r in roots) if rel]
        if anchored:
            forms.extend((True, rel) for rel in anchored)
        else:
            forms.append((False, candidate.replace(os.sep, "/").lstrip("/")))
    return forms


def _pattern_matches(has_slash, regexes, anchored, path_string):
    if not has_slash:
        name = path_string.rsplit("/", 1)[-1]
        return any(rx.match(name) for rx in regexes)
    # Each path is also tried as a directory, so `infra/**` covers `infra`
    # itself: `rm -rf infra` removes the whole tree the pattern protects.
    if anchored:
        candidates = [path_string]
    else:
        parts = path_string.split("/")
        candidates = ["/".join(parts[i:]) for i in range(len(parts))]
    return any(rx.match(c) or rx.match(c + "/") for rx in regexes for c in candidates)


def matching_pattern(forms, patterns):
    """The pattern line that marks this path sensitive, or None. Last match wins."""
    for anchored, path_string in forms:
        verdict = None
        for raw, negated, has_slash, regexes in patterns:
            if _pattern_matches(has_slash, regexes, anchored, path_string):
                verdict = None if negated else raw
        if verdict:
            return verdict
    return None


# --- Bash tokens -------------------------------------------------------------

def command_tokens(command, cwd):
    """Path-like tokens of the command text left after shell_data strips data.

    A token carrying glob characters is expanded against the filesystem the way
    the shell would (`cat .env*`), and kept as written too. Quoted redirect
    targets are collected before shell_data strips them as data (`echo x > ".env"`).
    """
    unheredoc = strip_heredoc_bodies(command)
    tokens = [double or single
              for double, single in QUOTED_REDIRECT_TARGET_PATTERN.findall(unheredoc)
              if double or single]
    text = strip_quoted_spans(unheredoc)
    for token in TOKEN_SPLIT_PATTERN.split(text):
        if not token:
            continue
        tokens.append(token)
        if GLOB_CHARS.search(token):
            matches = glob.iglob(os.path.join(cwd, token), recursive=True)
            tokens.extend(itertools.islice(matches, MAX_GLOB_MATCHES))
    return tokens


# --- Predicted paths ---------------------------------------------------------

def predicted_paths(task_id, tasks_dirs):
    """Backticked paths in the first column of the task's predicted-files table."""
    if not task_id:
        return set()
    for tasks_dir in tasks_dirs:
        body = read_guide_section(task_id, PREDICTED_LOOKUP, tasks_dir)
        if body is None:
            continue
        paths = set()
        for line in body.splitlines():
            cells = line.strip().strip("|").split("|")
            if not line.strip().startswith("|") or not cells:
                continue
            for span in BACKTICK_SPAN_PATTERN.findall(cells[0]):
                normalized = span.strip().replace("\\", "/")
                while normalized.startswith("./"):
                    normalized = normalized[2:]
                if normalized:
                    paths.add(normalized)
        return paths
    return set()


def is_predicted(forms, predicted):
    return any(anchored and path_string in predicted for anchored, path_string in forms)


# --- Online approvals (T314) -------------------------------------------------

def _rel(root, relpath):
    return os.path.join(root, *relpath.split("/"))


def approvals_file(root):
    return _rel(root, APPROVALS_RELPATH)


def pending_file(root, tool_use_id):
    """`.claude/state/pending/<tool_use_id>.json`, or None for an id unsafe as a file name."""
    if not isinstance(tool_use_id, str) or not TOOL_USE_ID_PATTERN.match(tool_use_id):
        return None
    return os.path.join(_rel(root, PENDING_RELDIR), tool_use_id + ".json")


def state_is_real(root, path):
    """True when `.claude/state` is a real folder of this project and `path` (no symlink) is in it."""
    state = _rel(root, STATE_RELDIR)
    if os.path.islink(state) or os.path.islink(path):
        return False
    real_state = os.path.join(os.path.realpath(root), *STATE_RELDIR.split("/"))
    return (os.path.realpath(state) == real_state
            and os.path.realpath(path).startswith(real_state + os.sep))


def read_json_in_state(root, path):
    """The JSON at `path` under a real `.claude/state`, else None."""
    if not state_is_real(root, path):
        return None
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except Exception:
        return None


def write_json_in_state(root, path, data):
    """Write `data` to `path` atomically; refuse a symlinked or foreign `.claude/state`."""
    for step in ("check", "create"):
        if step == "create":
            os.makedirs(os.path.dirname(path), exist_ok=True)
        if not state_is_real(root, path):
            raise OSError(f"{STATE_RELDIR} is not this project's own folder")
    tmp = f"{path}.{os.getpid()}.tmp"
    try:
        with open(tmp, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, sort_keys=True)
            f.write("\n")
        os.replace(tmp, path)
    finally:
        if os.path.lexists(tmp):
            os.remove(tmp)


def read_approvals(root):
    """The approval file as `{task_id: [key, ...]}`. Anything unusable -> `{}` (asks)."""
    data = read_json_in_state(root, approvals_file(root))
    if not isinstance(data, dict):
        return {}
    return {task: [p for p in paths if isinstance(p, str)]
            for task, paths in data.items()
            if isinstance(task, str) and isinstance(paths, list)}


def is_state_key(key):
    """Never approvable: anything under `.claude/state`, or a `sensitive-approvals.json*` file."""
    name = key.rsplit("/", 1)[-1]
    return (key == STATE_RELDIR or key.startswith(STATE_RELDIR + "/")
            or name == "sensitive-approvals.json" or name.startswith("sensitive-approvals.json."))


def file_key(path, cwd, root):
    """The one approval key of a file: root-relative under the root, else its absolute
    realpath. None for a directory or anything under `.claude/state` (never approvable).
    Built from the path as the OS sees it: no `\\` rewrite and no lexical `normpath` first,
    because on Linux `\\` is a file-name character (`a\\..\\b` is one new file, not `b`).
    None for a key that holds a `\\`: such a file is never approvable, it always asks."""
    real = os.path.realpath(os.path.join(cwd, path))
    if os.path.isdir(real) or "\\" in real:
        return None
    key = _within(real, os.path.realpath(root)) or real.replace(os.sep, "/")
    return None if is_state_key(key) else key


def sensitive_hits(tool_name, tool_input, cwd, roots, patterns):
    """`[(path, forms, pattern), ...]` for every sensitive path of the call.

    `.claude/state/**` is appended as the last pattern, so no `!` line in the
    project's list can unmark it (last match wins).
    """
    patterns = patterns + parse_patterns("\n".join(PROTECTED_PATTERNS))
    paths = candidate_paths(tool_name, tool_input, cwd)
    changes_dir = tool_name == "Bash" and any(t in DIR_CHANGE_TOKENS for t in paths)
    hits = []
    for path in paths:
        forms = path_forms(path, cwd, roots)
        pattern = matching_pattern(forms, patterns)
        if not pattern and changes_dir and any(
                anchored and f in STATE_PARENTS for anchored, f in forms):
            pattern = STATE_PATTERN
        if pattern:
            hits.append((path, forms, pattern))
    return hits


def written_key(path, cwd, root):
    """The path as written, in key form but unresolved: shows when a symlink moves the key.
    No `\\` rewrite, as in `file_key`."""
    absolute = os.path.normpath(os.path.join(cwd, path))
    return _within(absolute, root) or absolute.replace(os.sep, "/")


def is_online(roots):
    """Board mode online for any root; an unusable board.json counts as online (T157)."""
    return any(board_mode(r) != "local" for r in roots)


# --- Hook protocol -----------------------------------------------------------

def ask(reason):
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "ask",
            "permissionDecisionReason": f"{TAG} {reason}",
        }
    }))
    sys.exit(0)


def candidate_paths(tool_name, tool_input, cwd):
    if tool_name in ("Edit", "Write"):
        file_path = tool_input.get("file_path")
        if not isinstance(file_path, str) or not file_path.strip():
            raise ValueError(f"{tool_name} call has no file_path")
        return [file_path]
    command = tool_input.get("command")
    if not isinstance(command, str):
        raise ValueError("Bash call has no command string")
    return command_tokens(command, cwd)


def record_pending(root, tool_use_id, task_id, keys):
    """What this ask covers, for the PostToolUse recorder. A failed write only means
    no approval is recorded: the ask still goes out."""
    path = pending_file(root, tool_use_id)
    if path is None:
        return
    try:
        write_json_in_state(root, path, {"task_id": task_id, "keys": keys})
    except Exception:
        pass


def main():
    if IMPORT_ERROR is not None:
        ask(f"Sensitive-path check unavailable ({type(IMPORT_ERROR).__name__}: "
            f"{IMPORT_ERROR}) — cannot tell whether this call touches a sensitive "
            "path. Restore .claude/hooks/lib/.")

    try:
        event = json.load(sys.stdin)
        tool_name = event["tool_name"]
        tool_input = event["tool_input"]
        if not isinstance(tool_input, dict):
            raise ValueError("tool_input is not an object")
    except Exception as exc:
        ask(f"Malformed hook input ({type(exc).__name__}: {exc}) — cannot tell "
            "whether this call touches a sensitive path.")

    if tool_name not in ("Edit", "Write", "Bash"):
        sys.exit(0)

    root = project_dir()
    patterns_file = pattern_file_path(root)
    try:
        patterns = load_patterns(patterns_file)
    except Exception as exc:
        ask(f"Sensitive-path list unreadable ({patterns_file}: "
            f"{type(exc).__name__}) — cannot tell whether this call touches a "
            "sensitive path. Restore .claude/sensitive-paths.txt.")

    try:
        cwd = event.get("cwd") if isinstance(event.get("cwd"), str) else None
        cwd = cwd or os.getcwd()
        roots = [root, cwd]
        hits = sensitive_hits(tool_name, tool_input, cwd, roots, patterns)
        if not hits:
            sys.exit(0)

        task_id = resolve_task_id(event)
        board_text = is_online(roots)
        predicted = set() if board_text else predicted_paths(
            task_id, [os.path.join(root, "tasks"), os.path.join(cwd, "tasks")]
        )
        online_task = board_text and task_id
        approvable = tool_name in APPROVABLE_TOOLS
        approved = (set(read_approvals(root).get(task_id, []))
                    if online_task and approvable else set())
        unpredicted, keys = [], []
        for p, forms, pat in hits:
            protected = pat in PROTECTED_PATTERNS
            key = None if protected else file_key(p, cwd, root)
            if is_predicted(forms, predicted) and not protected:
                continue
            if key and key in approved:
                continue
            shown = p if not key or key == written_key(p, cwd, root) else f"{p} \u2192 {key}"
            unpredicted.append((shown, pat))
            if key and key not in keys:
                keys.append(key)
        if not unpredicted:
            sys.exit(0)
        if online_task and keys and approvable:
            record_pending(root, event.get("tool_use_id"), task_id, keys)
    except SystemExit:
        raise
    except Exception as exc:
        ask(f"Sensitive-path check failed ({type(exc).__name__}: {exc}) — "
            "asking rather than guessing.")

    listed = ", ".join(f"{p} (pattern `{pat}`)" for p, pat in unpredicted)
    owner = (f"the active task {task_id} does not list it under '## {PREDICTED_HEADING}'"
             if task_id else "no active task predicts it")
    if board_text:
        owner = ("board mode is online, where guides come from the board and cannot "
                 "pre-authorise a sensitive path")
        if task_id and keys and tool_name in APPROVABLE_TOOLS:
            owner += (f". After you approve, a later Edit or Write of these files in {task_id} "
                      "will not ask again")
        elif task_id and keys:
            owner += (f". A Bash call always asks; after you approve an Edit or Write of these "
                      f"files in {task_id}, a later Edit or Write of them will not ask again")
    ask(f"This {tool_name} call touches a sensitive path: {listed}. "
        f"Listed in .claude/sensitive-paths.txt and {owner}.")


if __name__ == "__main__":
    main()
