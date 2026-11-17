//! T108 (FR-117): the two built-in flows, as one table. `live::stage_of` (FR-083) looks a skill up in `STAGE_SKILLS` and
//! `GET /api/kit/workflow` serves the flows built from the same table, so the live stage map and the page's never differ.
//! Static: no file, store or network read. **easy-kit** is the kit's latest pipeline (`docs/claude-md/pipeline-stages.md`,
//! six steps; the agents are the base team at that step); **plain** is a session with no kit, placed by tool names,
//! sub-agent types and the first word of a Bash command.

use serde_json::{Value, json};

/// FR-083 (owner choice 2026-11-06): the fixed stage map, `(stage id, its skills)`. A skill not listed leaves a session's
/// stage unchanged. The ids are those of the live stream (`0.5` is Stage 0 · Define, `1.5` is Stage 1's agent team).
const STAGE_SKILLS: [(&str, &[&str]); 7] = [
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

/// What puts a session on a step. `skills` are not stored here: they are the `STAGE_SKILLS` of the step's `stages`.
struct Signals {
    stages: &'static [&'static str],
    agents: &'static [&'static str],
    tools: &'static [&'static str],
    commands: &'static [&'static str],
}

const NONE: Signals = Signals {
    stages: &[],
    agents: &[],
    tools: &[],
    commands: &[],
};

struct Step {
    id: &'static str,
    name: &'static str,
    parts: &'static [&'static str],
    agents: &'static [&'static str],
    signals: Signals,
}

/// A check between two steps; `before` is `None` for the last one (nothing comes after the merge).
struct Gate {
    after: &'static str,
    before: Option<&'static str>,
    text: &'static str,
}

struct Flow {
    id: &'static str,
    name: &'static str,
    steps: &'static [Step],
    gates: &'static [Gate],
}

const EASY_KIT: Flow = Flow {
    id: "easy-kit",
    name: "Easy kit",
    steps: &[
        Step {
            id: "0",
            name: "Stage 0 · Define",
            parts: &["Requirements", "Grilling", "Direction"],
            agents: &["Supervisor"],
            signals: Signals {
                stages: &["0.5"],
                ..NONE
            },
        },
        Step {
            id: "1",
            name: "Stage 1 · Set up",
            parts: &["Environment", "Agent team"],
            agents: &["Supervisor", "common-infrastructure"],
            signals: Signals {
                stages: &["1", "1.5"],
                agents: &["common-infrastructure"],
                ..NONE
            },
        },
        Step {
            id: "2",
            name: "Stage 2 · Plan",
            parts: &[],
            agents: &["Supervisor"],
            signals: Signals {
                stages: &["2"],
                ..NONE
            },
        },
        Step {
            id: "3",
            name: "Stage 3 · Execute",
            parts: &[],
            agents: &[
                "common-infrastructure",
                "backend-developer",
                "frontend-developer",
            ],
            signals: Signals {
                stages: &["3"],
                agents: &[
                    "common-infrastructure",
                    "backend-developer",
                    "frontend-developer",
                ],
                ..NONE
            },
        },
        Step {
            id: "4",
            name: "Stage 4 · Review",
            parts: &[],
            agents: &["Supervisor", "qa-expert"],
            signals: Signals {
                stages: &["4"],
                agents: &["qa-expert"],
                ..NONE
            },
        },
        Step {
            id: "5",
            name: "Stage 5 · Integrate",
            parts: &[],
            agents: &["qa-expert", "Supervisor"],
            signals: Signals {
                stages: &["5"],
                agents: &["qa-expert"],
                ..NONE
            },
        },
    ],
    gates: &[
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
    ],
};

const PLAIN: Flow = Flow {
    id: "plain",
    name: "Plain prompts (no kit)",
    steps: &[
        Step {
            id: "understand",
            name: "Understand",
            parts: &[],
            agents: &["Explore"],
            signals: Signals {
                agents: &["Explore"],
                tools: &["Read", "Grep", "Glob", "WebSearch", "WebFetch"],
                ..NONE
            },
        },
        Step {
            id: "plan",
            name: "Plan",
            parts: &[],
            agents: &["Plan"],
            signals: Signals {
                agents: &["Plan"],
                tools: &["EnterPlanMode", "ExitPlanMode", "TodoWrite"],
                ..NONE
            },
        },
        Step {
            id: "change",
            name: "Change",
            parts: &[],
            agents: &[],
            signals: Signals {
                tools: &["Edit", "Write", "MultiEdit", "NotebookEdit"],
                ..NONE
            },
        },
        Step {
            id: "check",
            name: "Check",
            parts: &[],
            agents: &[],
            signals: Signals {
                commands: &[
                    "cargo", "npm", "npx", "pnpm", "yarn", "pytest", "python", "python3", "go",
                    "make", "uv", "jest", "vitest",
                ],
                ..NONE
            },
        },
        Step {
            id: "ship",
            name: "Ship",
            parts: &[],
            agents: &[],
            signals: Signals {
                commands: &["git", "gh"],
                ..NONE
            },
        },
    ],
    gates: &[],
};

/// FR-083: the stage id of a skill; a skill not in the table is `None` (the session keeps its stage).
pub fn stage_of(skill: &str) -> Option<&'static str> {
    STAGE_SKILLS
        .iter()
        .find(|(_, skills)| skills.contains(&skill))
        .map(|(id, _)| *id)
}

/// A step's `signals.skills`: the skills of each of its FR-083 stages, in table order.
fn skills_of(stages: &[&str]) -> Vec<&'static str> {
    STAGE_SKILLS
        .iter()
        .filter(|(id, _)| stages.contains(id))
        .flat_map(|(_, skills)| skills.iter().copied())
        .collect()
}

fn flow_json(flow: &Flow) -> Value {
    json!({
        "id": flow.id,
        "name": flow.name,
        "steps": flow.steps.iter().map(|s| json!({
            "id": s.id, "name": s.name, "parts": s.parts, "agents": s.agents,
            "signals": {
                "stages": s.signals.stages,
                "skills": skills_of(s.signals.stages),
                "agents": s.signals.agents,
                "tools": s.signals.tools,
                "commands": s.signals.commands,
            },
        })).collect::<Vec<_>>(),
        "gates": flow.gates.iter().map(|g| json!({
            "after": g.after, "before": g.before, "text": g.text,
        })).collect::<Vec<_>>(),
    })
}

/// The body of `GET /api/kit/workflow`. `projects` (repo -> flow) is filled by the owner's flows (T109).
pub fn json() -> Value {
    json!({"flows": [flow_json(&EASY_KIT), flow_json(&PLAIN)], "projects": {}})
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

    fn flow(body: &Value, id: &str) -> Value {
        body["flows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == id)
            .unwrap_or_else(|| panic!("no flow {id}"))
            .clone()
    }

    fn strings(v: &Value) -> Vec<&str> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect()
    }

    /// T108 AC 1: two flows in order, every flow and step has its keys, `signals` has all five arrays.
    #[test]
    fn the_route_answers_two_flows_each_with_the_whole_shape() {
        let body = json();
        assert_eq!(body["projects"], serde_json::json!({}));
        let flows = body["flows"].as_array().unwrap();
        let ids: Vec<_> = flows.iter().map(|f| f["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["easy-kit", "plain"]);
        assert_eq!(flows[0]["name"], "Easy kit");
        assert_eq!(flows[1]["name"], "Plain prompts (no kit)");
        for flow in flows {
            assert!(flow["gates"].is_array());
            for step in flow["steps"].as_array().unwrap() {
                for key in ["id", "name"] {
                    assert!(step[key].is_string(), "{step}");
                }
                for key in ["parts", "agents"] {
                    assert!(step[key].is_array(), "{step}");
                }
                for key in ["stages", "skills", "agents", "tools", "commands"] {
                    assert!(step["signals"][key].is_array(), "{key} in {step}");
                }
            }
        }
    }

    /// T108 AC 2: easy-kit is the kit's latest six steps with their parts, agents and gates.
    #[test]
    fn easy_kit_is_the_six_steps_of_the_latest_pipeline_with_the_gates() {
        let kit = flow(&json(), "easy-kit");
        let steps = kit["steps"].as_array().unwrap();
        let got: Vec<_> = steps
            .iter()
            .map(|s| (s["id"].as_str().unwrap(), s["name"].as_str().unwrap()))
            .collect();
        assert_eq!(
            got,
            [
                ("0", "Stage 0 · Define"),
                ("1", "Stage 1 · Set up"),
                ("2", "Stage 2 · Plan"),
                ("3", "Stage 3 · Execute"),
                ("4", "Stage 4 · Review"),
                ("5", "Stage 5 · Integrate"),
            ]
        );
        let parts: Vec<_> = steps.iter().map(|s| strings(&s["parts"])).collect();
        assert_eq!(parts[0], ["Requirements", "Grilling", "Direction"]);
        assert_eq!(parts[1], ["Environment", "Agent team"]);
        assert!(parts[2..].iter().all(|p| p.is_empty()));
        let agents: Vec<_> = steps.iter().map(|s| strings(&s["agents"])).collect();
        assert_eq!(agents[0], ["Supervisor"]);
        assert_eq!(agents[1], ["Supervisor", "common-infrastructure"]);
        assert_eq!(agents[2], ["Supervisor"]);
        assert_eq!(
            agents[3],
            [
                "common-infrastructure",
                "backend-developer",
                "frontend-developer"
            ]
        );
        assert_eq!(agents[4], ["Supervisor", "qa-expert"]);
        assert_eq!(agents[5], ["qa-expert", "Supervisor"]);
        // signals.agents: the sub-agent types only, never the Supervisor; tools and commands empty
        assert_eq!(
            strings(&steps[1]["signals"]["agents"]),
            ["common-infrastructure"]
        );
        assert!(steps[0]["signals"]["agents"].as_array().unwrap().is_empty());
        for step in steps {
            assert!(step["signals"]["tools"].as_array().unwrap().is_empty());
            assert!(step["signals"]["commands"].as_array().unwrap().is_empty());
        }
        let stages: Vec<_> = steps
            .iter()
            .map(|s| strings(&s["signals"]["stages"]))
            .collect();
        assert_eq!(
            stages,
            [
                vec!["0.5"],
                vec!["1", "1.5"],
                vec!["2"],
                vec!["3"],
                vec!["4"],
                vec!["5"]
            ]
        );
        let gates: Vec<_> = kit["gates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| (g["after"].clone(), g["before"].clone(), g["text"].clone()))
            .collect();
        let want = [
            ("2", Some("3"), "No TASK_GUIDE, no Stage 3"),
            ("3", Some("4"), "Ready for Review starts the reviews"),
            ("4", Some("5"), "Reviews before verify"),
            ("5", None, "Verify passes before merge"),
        ];
        assert_eq!(gates.len(), want.len());
        for (got, (after, before, text)) in gates.iter().zip(want) {
            assert_eq!(got.0, after);
            assert_eq!(got.1, serde_json::json!(before));
            assert_eq!(got.2, text);
        }
    }

    /// T108 AC 2, both ways: `stage_of(skill) == Some(id)` exactly when the skill is in `signals.skills` of the step whose
    /// `signals.stages` holds `id`; and the union of the steps' stages is exactly the FR-083 ids.
    #[test]
    fn the_route_and_the_live_stage_map_are_the_same_table_both_ways() {
        let kit = flow(&json(), "easy-kit");
        let steps = kit["steps"].as_array().unwrap();
        let step_of = |id: &str| {
            steps
                .iter()
                .find(|s| strings(&s["signals"]["stages"]).contains(&id))
                .unwrap_or_else(|| panic!("no step holds stage {id}"))
        };
        let union: Vec<&str> = steps
            .iter()
            .flat_map(|s| strings(&s["signals"]["stages"]))
            .collect();
        assert_eq!(union, FR_083.map(|(id, _)| id));
        // route -> live
        let mut listed = 0;
        for step in steps {
            let stages = strings(&step["signals"]["stages"]);
            for skill in strings(&step["signals"]["skills"]) {
                listed += 1;
                let id = crate::live::stage_of(skill).unwrap_or_else(|| panic!("{skill} unmapped"));
                assert!(stages.contains(&id), "{skill} -> {id} not in {stages:?}");
            }
        }
        // live (FR-083, as written in the PRD) -> route
        let mut fr_083 = 0;
        for (id, skills) in FR_083 {
            for skill in skills {
                fr_083 += 1;
                assert_eq!(crate::live::stage_of(skill), Some(id), "{skill}");
                assert!(
                    strings(&step_of(id)["signals"]["skills"]).contains(skill),
                    "{skill} missing from the step of stage {id}"
                );
            }
        }
        assert_eq!(listed, fr_083, "a skill listed twice or not in FR-083");
        assert_eq!(crate::live::stage_of("not-a-kit-skill"), None);
    }

    /// T108 AC 3: plain is a session with no kit, five steps, the FR-117 signals, no gate.
    #[test]
    fn plain_is_five_steps_from_tools_agents_and_bash_first_words() {
        let plain = flow(&json(), "plain");
        let steps = plain["steps"].as_array().unwrap();
        let got: Vec<_> = steps
            .iter()
            .map(|s| (s["id"].as_str().unwrap(), s["name"].as_str().unwrap()))
            .collect();
        assert_eq!(
            got,
            [
                ("understand", "Understand"),
                ("plan", "Plan"),
                ("change", "Change"),
                ("check", "Check"),
                ("ship", "Ship")
            ]
        );
        let sig = |i: usize, k: &str| strings(&steps[i]["signals"][k]);
        assert_eq!(
            sig(0, "tools"),
            ["Read", "Grep", "Glob", "WebSearch", "WebFetch"]
        );
        assert_eq!(sig(0, "agents"), ["Explore"]);
        assert_eq!(strings(&steps[0]["agents"]), ["Explore"]);
        assert_eq!(
            sig(1, "tools"),
            ["EnterPlanMode", "ExitPlanMode", "TodoWrite"]
        );
        assert_eq!(sig(1, "agents"), ["Plan"]);
        assert_eq!(strings(&steps[1]["agents"]), ["Plan"]);
        assert_eq!(
            sig(2, "tools"),
            ["Edit", "Write", "MultiEdit", "NotebookEdit"]
        );
        assert_eq!(
            sig(3, "commands"),
            [
                "cargo", "npm", "npx", "pnpm", "yarn", "pytest", "python", "python3", "go", "make",
                "uv", "jest", "vitest"
            ]
        );
        assert_eq!(sig(4, "commands"), ["git", "gh"]);
        for i in [2, 3, 4] {
            assert!(steps[i]["agents"].as_array().unwrap().is_empty());
            assert!(sig(i, "agents").is_empty());
        }
        for step in steps {
            assert!(step["parts"].as_array().unwrap().is_empty());
            assert!(step["signals"]["stages"].as_array().unwrap().is_empty());
            assert!(step["signals"]["skills"].as_array().unwrap().is_empty());
        }
        assert!(plain["gates"].as_array().unwrap().is_empty());
    }

    /// T108 AC 3: every signal value passes the FR-081 label pattern, and no value sits in two steps of one flow
    /// (easy-kit's `agents` are the one exception by design: a sub-agent type works at more than one stage, and the view
    /// shows it on each step that lists it).
    #[test]
    fn signal_values_match_the_label_pattern_and_belong_to_one_step() {
        let ok = |v: &str| {
            let b = v.as_bytes();
            !b.is_empty()
                && b.len() <= 64
                && b[0].is_ascii_alphanumeric()
                && b.iter()
                    .all(|c| c.is_ascii_alphanumeric() || b":._-".contains(c))
        };
        let body = json();
        for flow in body["flows"].as_array().unwrap() {
            let mut seen = std::collections::HashSet::new();
            for step in flow["steps"].as_array().unwrap() {
                for key in ["stages", "skills", "agents", "tools", "commands"] {
                    for value in strings(&step["signals"][key]) {
                        assert!(ok(value), "{value} fails the label pattern");
                        if flow["id"] == "easy-kit" && key == "agents" {
                            continue;
                        }
                        assert!(
                            seen.insert((key, value)),
                            "{key} {value} is in two steps of {}",
                            flow["id"]
                        );
                    }
                }
            }
        }
        // in plain, the five kinds share one namespace too: a tool is never also a command
        let mut all = std::collections::HashSet::new();
        for step in flow(&body, "plain")["steps"].as_array().unwrap() {
            for key in ["stages", "skills", "agents", "tools", "commands"] {
                for value in strings(&step["signals"][key]) {
                    assert!(all.insert(value.to_string()), "{value} twice in plain");
                }
            }
        }
    }
}
