//! FR-132, DDR-0008 decision 4: the permission suggestions Claude Code offers with a prompt ("allow, and don't ask again").
//!
//! The CLI's own suggestion is the only thing an answer may name: the app keeps the list it was given for that prompt, the
//! card shows each one as a short summary and where it lands, and an answer carries only an index into that list. The
//! suggestion goes back to Claude Code as it came (`updatedPermissions`); Claude Code writes the rule itself. The app
//! builds no rule and opens no settings file. A suggestion the card cannot show whole (a shape it does not know, a text
//! that would be cut, a destination it cannot name) is not offered at all: the owner must see what they allow.

use easy_personal_core::live::clean_block;
use serde_json::Value;

/// Suggestions kept per prompt (the CLI offers one to three).
const MAX_KEPT: usize = 8;
/// Rules or folders named in one suggestion; more are not offered (they could not all be shown).
const MAX_NAMED: usize = 5;
/// The longest summary; a longer one is not offered.
const SUMMARY_MAX: usize = 600;

/// The suggestions of a prompt, unchanged: the first [`MAX_KEPT`] of an array; anything else is none.
pub(crate) fn keep(raw: Option<&Value>) -> Vec<Value> {
    raw.and_then(Value::as_array)
        .map(|list| list.iter().take(MAX_KEPT).cloned().collect())
        .unwrap_or_default()
}

/// Where a rule lands, in the words of the button (FR-132): a session, a project, or the owner everywhere.
fn lands(destination: &str) -> Option<&'static str> {
    match destination {
        "session" => Some("this session"),
        "localSettings" => Some("this project (just you)"),
        "projectSettings" => Some("this project (shared)"), // the committed project file, not only the owner's
        "userSettings" => Some("you, everywhere"),
        _ => None,
    }
}

/// One text of a summary, cleaned for display and whole: `None` when the cut removed anything.
fn whole(raw: &str) -> Option<String> {
    let (text, complete) = clean_block(raw, SUMMARY_MAX);
    complete.then_some(text)
}

fn strings(value: &Value) -> Option<Vec<&str>> {
    let list = value.as_array()?;
    if list.is_empty() || list.len() > MAX_NAMED {
        return None;
    }
    list.iter().map(Value::as_str).collect()
}

/// `(where, summary)` of one suggestion, or `None` when it is not offered: an allow rule (`addRules` with behavior
/// `allow`), a mode switch (not to `bypassPermissions`) or added folders, with a destination the card can name.
fn view(suggestion: &Value) -> Option<(&'static str, String)> {
    let where_ = lands(suggestion.get("destination")?.as_str()?)?;
    let summary = match suggestion.get("type")?.as_str()? {
        "addRules" => {
            if suggestion.get("behavior")?.as_str()? != "allow" {
                return None;
            }
            let rules = suggestion.get("rules")?.as_array()?;
            if rules.is_empty() || rules.len() > MAX_NAMED {
                return None;
            }
            let mut named = Vec::new();
            for rule in rules {
                let tool = whole(rule.get("toolName")?.as_str().filter(|t| !t.is_empty())?)?;
                named.push(match rule.get("ruleContent") {
                    None | Some(Value::Null) => tool,
                    Some(Value::String(content)) => format!("{tool}({})", whole(content)?),
                    Some(_) => return None,
                });
            }
            format!("don't ask again for {}", named.join(", "))
        }
        "setMode" => {
            let mode = suggestion.get("mode")?.as_str()?;
            if mode.is_empty() || mode == "bypassPermissions" {
                return None;
            }
            format!("switch to {} mode", whole(mode)?)
        }
        "addDirectories" => {
            let folders: Vec<String> = strings(suggestion.get("directories")?)?
                .into_iter()
                .map(whole)
                .collect::<Option<_>>()?;
            format!("add the folder {}", folders.join(", "))
        }
        _ => return None,
    };
    Some((where_, summary))
}

/// What a card shows: `{index, where, summary}` for each suggestion that is offered. `index` is the position in the kept
/// list, which is what an answer names.
pub(crate) fn views(kept: &[Value]) -> Vec<Value> {
    kept.iter()
        .enumerate()
        .filter_map(|(index, s)| {
            let (where_, summary) = view(s)?;
            Some(serde_json::json!({"index": index, "where": where_, "summary": summary}))
        })
        .collect()
}

/// The suggestion at `index`, only when the card showed it.
pub(crate) fn offered(kept: &[Value], index: usize) -> Option<&Value> {
    let suggestion = kept.get(index)?;
    view(suggestion).map(|_| suggestion)
}

/// The offered suggestion that switches to `acceptEdits` (the plan card's second button), if the CLI gave one.
pub(crate) fn accept_edits(kept: &[Value]) -> Option<&Value> {
    kept.iter().find(|s| {
        s.get("type").and_then(Value::as_str) == Some("setMode")
            && s.get("mode").and_then(Value::as_str) == Some("acceptEdits")
            && view(s).is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rule(content: &str, destination: &str) -> Value {
        json!({"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": content}],
               "behavior": "allow", "destination": destination})
    }

    #[test]
    fn a_rule_a_mode_and_a_folder_are_offered_with_where_they_land() {
        let kept = keep(Some(&json!([
            rule("git status:*", "localSettings"),
            {"type": "setMode", "mode": "acceptEdits", "destination": "session"},
            {"type": "addDirectories", "directories": ["/work/a"], "destination": "userSettings"},
        ])));
        let seen = views(&kept);
        assert_eq!(
            seen,
            vec![
                json!({"index": 0, "where": "this project (just you)", "summary": "don't ask again for Bash(git status:*)"}),
                json!({"index": 1, "where": "this session", "summary": "switch to acceptEdits mode"}),
                json!({"index": 2, "where": "you, everywhere", "summary": "add the folder /work/a"}),
            ]
        );
        assert_eq!(lands("projectSettings"), Some("this project (shared)"));
        assert_eq!(lands("localSettings"), Some("this project (just you)"));
        assert_eq!(accept_edits(&kept), Some(&kept[1]));
    }

    #[test]
    fn what_the_card_cannot_name_or_show_whole_is_not_offered() {
        let long = "x".repeat(SUMMARY_MAX + 1);
        let kept = keep(Some(&json!([
            rule("a", "cliArg"),                                                  // 0: a destination it cannot name
            {"type": "addRules", "rules": [{"toolName": "Bash"}], "behavior": "deny", "destination": "session"}, // 1: not an allow
            rule(&long, "session"),                                               // 2: would be cut
            {"type": "setMode", "mode": "bypassPermissions", "destination": "session"}, // 3
            {"type": "replaceRules", "rules": [], "behavior": "allow", "destination": "session"}, // 4: unknown type
            {"type": "addRules", "rules": [], "behavior": "allow", "destination": "session"}, // 5: no rules
            {"type": "addRules", "rules": [{"toolName": 3}], "behavior": "allow", "destination": "session"}, // 6
            "text",                                                               // 7
        ])));
        assert_eq!(kept.len(), 8);
        assert!(views(&kept).is_empty(), "{:?}", views(&kept));
        for i in 0..8 {
            assert!(offered(&kept, i).is_none(), "{i}");
        }
        assert!(accept_edits(&kept).is_none());
    }

    #[test]
    fn a_hidden_character_shows_as_an_escape_and_a_rule_without_content_is_its_tool() {
        let kept = keep(Some(&json!([
            rule("ls\u{202e}x", "session"),
            {"type": "addRules", "rules": [{"toolName": "WebFetch"}], "behavior": "allow", "destination": "session"},
        ])));
        let seen = views(&kept);
        assert_eq!(seen[0]["summary"], "don't ask again for Bash(ls\\u{202e}x)");
        assert_eq!(seen[1]["summary"], "don't ask again for WebFetch");
    }

    #[test]
    fn the_list_is_capped_and_not_an_array_is_none() {
        let many: Vec<Value> = (0..20).map(|i| rule(&format!("c{i}"), "session")).collect();
        assert_eq!(keep(Some(&Value::Array(many))).len(), MAX_KEPT);
        assert!(keep(Some(&json!({"type": "addRules"}))).is_empty());
        assert!(keep(None).is_empty());
    }
}
