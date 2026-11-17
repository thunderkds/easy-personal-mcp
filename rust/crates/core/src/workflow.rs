//! T108 (FR-117): the kit's fixed Stage 0-5 pipeline, as one table. `live::stage_of` (FR-083) looks a skill up in it and
//! `GET /api/kit/workflow` serves it, so the live stage map and the page's map never differ. Static: no file, store or
//! network read. The ids are the ones of FR-083 (`0.5` is Stage 0 · Define); the agents are the kit's base team at that
//! stage (`AGENTS.md` § Stage 0-5 Agentic Pipeline).

use serde_json::{Value, json};

pub struct Stage {
    pub id: &'static str,
    pub name: &'static str,
    pub skills: &'static [&'static str],
    pub agents: &'static [&'static str],
}

/// A check between two stages; `before` is `None` for the last one (nothing comes after the merge).
pub struct Gate {
    pub after: &'static str,
    pub before: Option<&'static str>,
    pub text: &'static str,
}

pub const STAGES: [Stage; 7] = [
    Stage {
        id: "0.5",
        name: "Stage 0 · Define",
        skills: &["brainstorming", "ideate"],
        agents: &["Supervisor"],
    },
    Stage {
        id: "1",
        name: "Stage 1 · Set up",
        skills: &["git-guardrails-claude-code", "map-codebase"],
        agents: &["Supervisor", "common-infrastructure"],
    },
    Stage {
        id: "1.5",
        name: "Stage 1 · Agent team",
        skills: &["craft-agent"],
        agents: &["Supervisor"],
    },
    Stage {
        id: "2",
        name: "Stage 2 · Plan",
        skills: &["grill-with-docs", "to-issues"],
        agents: &["Supervisor"],
    },
    Stage {
        id: "3",
        name: "Stage 3 · Execute",
        skills: &[
            "tdd",
            "bugfix",
            "diagnose",
            "craft-spawn-prompt",
            "migration-safety",
        ],
        agents: &[
            "common-infrastructure",
            "backend-developer",
            "frontend-developer",
        ],
    },
    Stage {
        id: "4",
        name: "Stage 4 · Review",
        skills: &[
            "code-review",
            "security-review",
            "blast-radius",
            "html-report",
        ],
        agents: &["Supervisor", "qa-expert"],
    },
    Stage {
        id: "5",
        name: "Stage 5 · Integrate",
        skills: &["verify", "ship", "delivery-report"],
        agents: &["qa-expert", "Supervisor"],
    },
];

pub const GATES: [Gate; 4] = [
    Gate {
        after: "2",
        before: Some("3"),
        text: "No TASK_GUIDE, no Stage 3",
    },
    Gate {
        after: "3",
        before: Some("4"),
        text: "Ready for Review starts the reviews",
    },
    Gate {
        after: "4",
        before: Some("5"),
        text: "Reviews before verify",
    },
    Gate {
        after: "5",
        before: None,
        text: "Verify passes before merge",
    },
];

/// FR-083: the stage id of a skill; a skill not in the table is `None` (the session keeps its stage).
pub fn stage_of(skill: &str) -> Option<&'static str> {
    STAGES
        .iter()
        .find(|stage| stage.skills.contains(&skill))
        .map(|stage| stage.id)
}

/// The body of `GET /api/kit/workflow`.
pub fn json() -> Value {
    json!({
        "stages": STAGES.iter().map(|s| json!({
            "id": s.id, "name": s.name, "skills": s.skills, "agents": s.agents,
        })).collect::<Vec<_>>(),
        "gates": GATES.iter().map(|g| json!({
            "after": g.after, "before": g.before, "text": g.text,
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-083 (`PRD.md`), as written there: the fixed stage map, in the table's own order.
    const FR_083: [(&str, &[&str]); 7] = [
        ("0.5", &["brainstorming", "ideate"]),
        ("1", &["git-guardrails-claude-code", "map-codebase"]),
        ("1.5", &["craft-agent"]),
        ("2", &["grill-with-docs", "to-issues"]),
        (
            "3",
            &[
                "tdd",
                "bugfix",
                "diagnose",
                "craft-spawn-prompt",
                "migration-safety",
            ],
        ),
        (
            "4",
            &[
                "code-review",
                "security-review",
                "blast-radius",
                "html-report",
            ],
        ),
        ("5", &["verify", "ship", "delivery-report"]),
    ];

    /// T108 AC 2, both ways: every skill the live tail maps is in the route's stage of the same id, and every skill the
    /// route lists is mapped to that id by the live tail.
    #[test]
    fn the_route_and_the_live_stage_map_are_the_same_table_both_ways() {
        let body = json();
        let stages = body["stages"].as_array().unwrap();
        assert_eq!(stages.len(), FR_083.len());
        for (stage, (id, skills)) in stages.iter().zip(FR_083) {
            assert_eq!(stage["id"], id);
            let listed: Vec<&str> = stage["skills"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap())
                .collect();
            // route -> live
            for skill in &listed {
                assert_eq!(crate::live::stage_of(skill), Some(id), "{skill}");
            }
            // live (FR-083) -> route
            assert_eq!(listed, skills, "stage {id}");
            for skill in skills {
                assert_eq!(crate::live::stage_of(skill), Some(id), "{skill}");
            }
        }
        assert_eq!(crate::live::stage_of("not-a-kit-skill"), None);
    }

    #[test]
    fn a_skill_is_on_one_stage_only_and_every_gate_names_a_known_stage() {
        let mut seen = std::collections::HashSet::new();
        for stage in &STAGES {
            for skill in stage.skills {
                assert!(seen.insert(*skill), "{skill} is on two stages");
            }
            assert!(!stage.agents.is_empty(), "stage {}", stage.id);
        }
        let known = |id: &str| STAGES.iter().any(|s| s.id == id);
        for gate in &GATES {
            assert!(known(gate.after) && gate.before.is_none_or(known));
        }
    }
}
