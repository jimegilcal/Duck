use serde::{Deserialize, Serialize};
use tauri::async_runtime;
#[cfg(target_os = "macos")]
use tauri::Manager;
#[cfg(target_os = "macos")]
use {
    rusqlite::{Connection, OpenFlags},
    sha2::{Digest, Sha256},
    std::collections::HashMap,
};

#[cfg(target_os = "macos")]
extern "C" {
    fn duck_running_agents() -> u32;
    fn getuid() -> u32;
}

const MAX_FILE_BYTES: u64 = 60_000;
const MAX_AGE_MS: i64 = 43_200_000;
const CLOSED_AGE_MS: i64 = 3_600_000;
const MAX_SESSIONS: usize = 20;

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AgentStatus {
    Ready,
    Working,
    Approval,
    Finished,
    Interrupted,
    Closed,
}

#[derive(Deserialize, Serialize, Default)]
pub struct Todo {
    title: String,
    status: String,
    #[serde(default)]
    active: String,
}

#[derive(Deserialize, Serialize, Default)]
pub struct SubAgent {
    id: String,
    title: String,
    #[serde(default)]
    kind: String,
    status: String,
    started: i64,
    #[serde(default)]
    ended: Option<i64>,
}

#[derive(Deserialize, Serialize, Default)]
pub struct TimelineItem {
    t: i64,
    text: String,
}

#[derive(Deserialize, Serialize)]
pub struct AgentUpdate {
    provider: String,
    status: AgentStatus,
    project: String,
    timestamp: i64,
    #[serde(default)]
    session: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    goal: String,
    #[serde(default)]
    todos: Vec<Todo>,
    #[serde(default)]
    agents: Vec<SubAgent>,
    #[serde(default)]
    timeline: Vec<TimelineItem>,
}

fn accept(update: &mut AgentUpdate, now: i64) -> bool {
    if !["codex", "claude-code"].contains(&update.provider.as_str())
        || !(now - MAX_AGE_MS..=now + 10_000).contains(&update.timestamp)
    {
        return false;
    }
    if matches!(update.status, AgentStatus::Closed) && now - update.timestamp > CLOSED_AGE_MS {
        return false;
    }
    update.todos.truncate(30);
    update.agents.truncate(40);
    update.timeline.truncate(25);
    true
}

#[cfg(target_os = "macos")]
fn read_hook_updates() -> Vec<AgentUpdate> {
    use std::{fs, os::unix::fs::MetadataExt, path::PathBuf};

    let uid = unsafe { getuid() };
    let folder = PathBuf::from(format!("/tmp/duck-agent-events-{uid}"));
    let Ok(owner) = fs::symlink_metadata(&folder) else { return Vec::new() };
    if !owner.is_dir() || owner.uid() != uid || owner.mode() & 0o077 != 0 {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(folder) else { return Vec::new() };
    let mut sessions = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") { continue; }
        let Ok(owner) = fs::symlink_metadata(&path) else { continue };
        if !owner.is_file() || owner.uid() != uid || owner.len() > MAX_FILE_BYTES { continue; }
        let Ok(bytes) = fs::read(path) else { continue };
        let Ok(update) = serde_json::from_slice::<AgentUpdate>(&bytes) else { continue };
        sessions.push(update);
    }
    sessions
}

#[cfg(target_os = "macos")]
fn codex_thread_titles() -> HashMap<String, String> {
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        return HashMap::new();
    };
    let Ok(connection) = Connection::open_with_flags(
        home.join(".codex/state_5.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return HashMap::new();
    };
    let Ok(mut statement) = connection.prepare(
        "SELECT id, COALESCE(NULLIF(name, ''), NULLIF(title, ''), '') FROM threads WHERE archived = 0",
    ) else {
        return HashMap::new();
    };
    let Ok(rows) = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) else {
        return HashMap::new();
    };
    rows.filter_map(Result::ok)
        .filter_map(|(id, title)| {
            let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
            if title.is_empty() {
                return None;
            }
            let hash = format!("{:x}", Sha256::digest(format!("codex:{id}")));
            Some((hash[..12].to_owned(), title.chars().take(100).collect()))
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn add_codex_titles(sessions: &mut [AgentUpdate]) {
    let titles = codex_thread_titles();
    for session in sessions.iter_mut().filter(|entry| entry.provider == "codex") {
        if let Some(title) = titles.get(&session.session) {
            session.title.clone_from(title);
        }
    }
}

#[cfg(target_os = "macos")]
fn merge_codex_turn(sessions: &mut Vec<AgentUpdate>, turn: crate::codex_activity::ObservedTurn) {
    let timestamp = if turn.status == AgentStatus::Working {
        turn.activity_at
    } else {
        turn.event_at
    };
    if let Some(update) = sessions.iter_mut().find(|entry| entry.provider == "codex" && entry.session == turn.session) {
        if turn.event_at > update.timestamp {
            update.status = turn.status;
            update.timestamp = timestamp;
        } else if turn.status == AgentStatus::Working && turn.activity_at > update.timestamp {
            // PermissionRequest has no matching "permission granted" hook. Fresh
            // rollout activity is the reliable signal that Codex resumed work.
            update.status = AgentStatus::Working;
            update.timestamp = turn.activity_at;
        }
    } else {
        sessions.push(AgentUpdate {
            provider: "codex".into(),
            status: turn.status,
            project: turn.project,
            timestamp,
            session: turn.session,
            title: String::new(),
            goal: String::new(),
            todos: Vec::new(),
            agents: Vec::new(),
            timeline: Vec::new(),
        });
    }
}

#[tauri::command]
pub async fn agent_updates(app: tauri::AppHandle) -> Vec<AgentUpdate> {
    #[cfg(target_os = "macos")]
    {
        let usage_tracking = app.state::<crate::desktop::DesktopState>()
            .lock().is_ok_and(|desktop| desktop.settings.usage_tracking);
        async_runtime::spawn_blocking(move || {
            let now = chrono::Utc::now().timestamp_millis();
            let mut sessions = read_hook_updates();
            if usage_tracking {
                for turn in crate::codex_activity::recent_turns() {
                    merge_codex_turn(&mut sessions, turn);
                }
            }
            add_codex_titles(&mut sessions);
            sessions.retain_mut(|update| accept(update, now));
            sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            sessions.truncate(MAX_SESSIONS);
            sessions
        }).await.unwrap_or_default()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Vec::new()
    }
}

#[tauri::command]
pub fn running_agents() -> Vec<&'static str> {
    #[cfg(target_os = "macos")]
    {
        let presence = unsafe { duck_running_agents() };
        ["codex", "claude-code"]
            .into_iter()
            .enumerate()
            .filter_map(|(index, id)| (presence & (1 << index) != 0).then_some(id))
            .collect()
    }
    #[cfg(not(target_os = "macos"))]
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> AgentUpdate {
        serde_json::from_str(json).expect("válido")
    }

    #[test]
    fn old_codex_signal_still_parses() {
        let now = 1_700_000_000_000;
        let mut update = parse(&format!(r#"{{"provider":"codex","status":"working","project":"Duck","timestamp":{now}}}"#));
        assert!(accept(&mut update, now));
        assert!(update.todos.is_empty() && update.agents.is_empty());
    }

    #[test]
    fn claude_code_summary_parses() {
        let now = 1_700_000_000_000;
        let mut update = parse(&format!(
            r#"{{"provider":"claude-code","status":"working","project":"Duck","timestamp":{now},"session":"ab12",
            "goal":"Arreglar el chat","todos":[{{"title":"Leer","status":"completed","active":"Leyendo"}}],
            "agents":[{{"id":"1","title":"Buscar","kind":"Explore","status":"running","started":{now},"ended":null}}],
            "timeline":[{{"t":{now},"text":"Nueva petición"}}]}}"#));
        assert!(accept(&mut update, now));
        assert_eq!((update.todos.len(), update.agents.len(), update.timeline.len()), (1, 1, 1));
    }

    #[test]
    fn rejects_other_providers_and_stale_sessions() {
        let now = 1_700_000_000_000;
        let mut other = parse(&format!(r#"{{"provider":"chatgpt","status":"working","project":"x","timestamp":{now}}}"#));
        assert!(!accept(&mut other, now));
        let mut old = parse(&format!(r#"{{"provider":"codex","status":"working","project":"x","timestamp":{}}}"#, now - MAX_AGE_MS - 1));
        assert!(!accept(&mut old, now));
        let mut closed = parse(&format!(r#"{{"provider":"codex","status":"closed","project":"x","timestamp":{}}}"#, now - CLOSED_AGE_MS - 1));
        assert!(!accept(&mut closed, now));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn shows_a_recent_codex_result_after_the_turn_finishes() {
        use crate::codex_activity::ObservedTurn;

        let start = 1_700_000_000_000;
        let observed = |status, timestamp| ObservedTurn {
            session: "abc123".into(),
            project: "Duck".into(),
            status,
            event_at: timestamp,
            activity_at: timestamp,
        };
        let mut sessions = Vec::new();
        merge_codex_turn(&mut sessions, observed(AgentStatus::Working, start));
        merge_codex_turn(&mut sessions, observed(AgentStatus::Finished, start + 1000));
        assert_eq!(sessions.len(), 1);
        assert!(matches!(sessions[0].status, AgentStatus::Finished));
        sessions.clear();
        merge_codex_turn(&mut sessions, observed(AgentStatus::Finished, start + 1000));
        assert_eq!(sessions.len(), 1);
        assert!(matches!(sessions[0].status, AgentStatus::Finished));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn fresh_rollout_activity_clears_a_satisfied_approval() {
        use crate::codex_activity::ObservedTurn;

        let start = 1_700_000_000_000;
        let mut sessions = vec![parse(&format!(
            r#"{{"provider":"codex","status":"approval","project":"Duck","timestamp":{},"session":"abc123"}}"#,
            start + 5_000,
        ))];
        merge_codex_turn(&mut sessions, ObservedTurn {
            session: "abc123".into(),
            project: "Duck".into(),
            status: AgentStatus::Working,
            event_at: start,
            activity_at: start + 6_000,
        });
        assert!(matches!(sessions[0].status, AgentStatus::Working));
        assert_eq!(sessions[0].timestamp, start + 6_000);
    }
}
