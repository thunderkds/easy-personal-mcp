"""Project edits to vendored Easy Kit files that a kit update must not drop (HUN-205).

A kit update overwrites edited kit files and keeps a `.bak-<commit>` copy. This test fails when
an update drops the project's own Stage 4 step: always run the easy-verifier `score`.
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_stage4_always_runs_the_easy_verifier_score():
    agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
    stages = (ROOT / "docs/claude-md/pipeline-stages.md").read_text(encoding="utf-8")

    stage4 = next(line for line in agents.splitlines() if line.startswith("- **Stage 4** (Review)"))
    assert 'easy-verifier MCP `score` (`scope="changes"`, advisory) always' in stage4
    assert "4b. **easy-verifier score** (always; advisory" in stages
    assert 'scope="changes"' in stages and 'ref="main...<task branch>"' in stages
