"""Tiny stdio MCP server for the T102 spike: one tool `permission_prompt`.

Logs every call's raw arguments (JSON lines) to $PERM_LOG and answers from $PERM_ANSWERS (a JSON file read per call:
{"<tool_name>": <result object or {"_raw": "<text>"}>, "*": <default>}). The result object is returned as the tool's
single text content (that is the documented route: the tool result text is the JSON decision).
"""
import json, os, sys, time

LOG = os.environ["PERM_LOG"]
ANS = os.environ["PERM_ANSWERS"]
DELAY = float(os.environ.get("PERM_DELAY", "0"))


def log(obj):
    with open(LOG, "a") as f:
        f.write(json.dumps({"t": time.time(), **obj}) + "\n")


def send(msg):
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


for line in sys.stdin:
    try:
        m = json.loads(line)
    except Exception:
        continue
    method, mid = m.get("method"), m.get("id")
    log({"rpc": method, "has_id": mid is not None})
    if method == "initialize":
        send({"jsonrpc": "2.0", "id": mid, "result": {"protocolVersion": m["params"].get("protocolVersion", "2025-06-18"),
              "capabilities": {"tools": {}}, "serverInfo": {"name": "perm", "version": "0"}}})
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": mid, "result": {"tools": [{
            "name": "permission_prompt", "description": "Decides permission prompts (spike).",
            "inputSchema": {"type": "object", "properties": {"tool_name": {"type": "string"}, "input": {"type": "object"},
                            "tool_use_id": {"type": "string"}}, "required": ["tool_name", "input"]}}]}})
    elif method == "tools/call":
        args = m["params"].get("arguments", {})
        log({"call": m["params"].get("name"), "arguments": args, "meta_keys": sorted((m["params"].get("_meta") or {}).keys())})
        time.sleep(DELAY)
        answers = json.load(open(ANS))
        a = answers.get(args.get("tool_name"), answers.get("*"))
        text = a["_raw"] if isinstance(a, dict) and "_raw" in a else json.dumps(a)
        if isinstance(a, dict) and a.get("behavior") == "allow" and a.get("_echo_input"):
            text = json.dumps({"behavior": "allow", "updatedInput": args.get("input")})
        if isinstance(a, dict) and a.get("_ask"):  # AskUserQuestion: answer the first question with its 2nd option label
            inp = args.get("input", {})
            q = inp["questions"][0]
            label = a.get("_label") or (q["options"][1]["label"] if len(q.get("options", [])) > 1 else "x")
            text = json.dumps({"behavior": "allow", "updatedInput": {**inp, "answers": {q["question"]: label}}})
            log({"answered": label})
        send({"jsonrpc": "2.0", "id": mid, "result": {"content": [{"type": "text", "text": text}]}})
    elif mid is not None:
        send({"jsonrpc": "2.0", "id": mid, "result": {}})
