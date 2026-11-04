"""Improvement plan (FR-007): the fired rules grouped by priority, each with its fix line."""

from datetime import datetime

from easy_personal.core import rules as rule_engine


def improvement_plan(
    conn, now: datetime | None = None, rules=None, workspace: str | None = None, account: str | None = None
) -> dict:
    groups: dict[int, list[dict]] = {}
    for hit in rule_engine.evaluate(conn, now, rules, workspace, account)["hits"]:
        item = {k: hit[k] for k in ("rule_id", "label", "measured", "threshold", "fix", "title", "why", "steps", "summary", "causes", "diagnosis")}
        for key in ("detail", "cache_read_share"):
            if key in hit:
                item[key] = hit[key]
        groups.setdefault(hit["priority"], []).append(item)
    return {"groups": [{"priority": p, "items": groups[p]} for p in sorted(groups)]}
