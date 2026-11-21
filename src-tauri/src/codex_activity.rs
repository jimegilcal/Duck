use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    agents::AgentStatus,
    usage::{collect, parse_ms, project_name},
};

const RECENT: Duration = Duration::from_secs(10 * 60);
const MAX_LOGS: usize = 20;
const MAX_HEADER_BYTES: u64 = 256 * 1024;
const MAX_TAIL_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) struct ObservedTurn {
    pub session: String,
    pub project: String,
    pub status: AgentStatus,
    pub event_at: i64,
    pub activity_at: i64,
}

struct CachedLog {
    len: u64,
    modified: SystemTime,
    offset: u64,
    session: String,
    project: String,
    turn: Option<(AgentStatus, i64)>,
}

#[derive(Deserialize)]
struct Record<T> {
    #[serde(rename = "type")]
    kind: String,
    timestamp: Option<String>,
    payload: T,
}

#[derive(Deserialize)]
struct SessionMeta {
    id: String,
    cwd: String,
}

#[derive(Deserialize)]
struct TurnEvent {
    #[serde(rename = "type")]
    kind: String,
}

fn read_turn(line: &[u8]) -> Option<(AgentStatus, i64)> {
    // The rollout starts each event with its metadata. Skip message and tool records.
    let prefix = &line[..line.len().min(256)];
    let event_types: [&[u8]; 3] = [b"\"task_started\"", b"\"task_complete\"", b"\"turn_aborted\""];
    if !event_types.iter().any(|kind| prefix.windows(kind.len()).any(|word| word == *kind)) {
        return None;
    }
    let event = serde_json::from_slice::<Record<TurnEvent>>(line).ok()?;
    if event.kind != "event_msg" {
        return None;
    }
    let status = match event.payload.kind.as_str() {
        "task_started" => AgentStatus::Working,
        "task_complete" => AgentStatus::Finished,
        "turn_aborted" => AgentStatus::Interrupted,
        _ => return None,
    };
    Some((status, parse_ms(event.timestamp.as_deref()?)?))
}

fn read_log(
    mut reader: impl BufRead + Seek,
    len: u64,
    modified: SystemTime,
    previous: Option<CachedLog>,
) -> Option<CachedLog> {
    let mut line = Vec::new();
    let mut state = match previous.filter(|previous| previous.len <= len) {
        Some(previous) => previous,
        None => {
            let offset = (&mut reader).take(MAX_HEADER_BYTES).read_until(b'\n', &mut line).ok()? as u64;
            let meta = serde_json::from_slice::<Record<SessionMeta>>(&line).ok()?;
            if meta.kind != "session_meta" || uuid::Uuid::parse_str(&meta.payload.id).is_err() {
                return None;
            }
            // Use the same pseudonymous session key as the hook.
            let hash = format!("{:x}", Sha256::digest(format!("codex:{}", meta.payload.id)));
            CachedLog {
                len,
                modified,
                offset,
                session: hash[..12].to_owned(),
                project: project_name(&meta.payload.cwd),
                turn: None,
            }
        }
    };
    let start = state.offset.max(len.saturating_sub(MAX_TAIL_BYTES));
    let mut partial_line = false;
    if start > state.offset {
        state.turn = None;
        reader.seek(SeekFrom::Start(start - 1)).ok()?;
        let mut previous_byte = [0u8];
        reader.read_exact(&mut previous_byte).ok()?;
        partial_line = previous_byte[0] != b'\n';
    } else {
        reader.seek(SeekFrom::Start(start)).ok()?;
    }

    let mut reader = reader.take(len.saturating_sub(start));
    state.offset = start;
    if partial_line {
        line.clear();
        state.offset += reader.read_until(b'\n', &mut line).ok()? as u64;
    }
    loop {
        line.clear();
        let bytes = reader.read_until(b'\n', &mut line).ok()?;
        if bytes == 0 || line.last() != Some(&b'\n') {
            break;
        }
        state.offset += bytes as u64;
        if let Some(turn) = read_turn(&line) {
            state.turn = Some(turn);
        }
    }
    state.len = len;
    state.modified = modified;
    Some(state)
}

pub(crate) fn recent_turns() -> Vec<ObservedTurn> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedLog>>> = OnceLock::new();
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else { return Vec::new() };
    let mut paths = Vec::new();
    let cutoff = SystemTime::now() - RECENT;
    let cutoff_ms = chrono::DateTime::<chrono::Utc>::from(cutoff).timestamp_millis();
    collect(&home.join(".codex/sessions"), 0, cutoff, &mut paths);
    paths.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    paths.truncate(MAX_LOGS);

    let mut cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()))
        .lock().unwrap_or_else(|error| error.into_inner());
    cache.retain(|path, _| paths.iter().any(|(candidate, _, _)| path == candidate));

    let mut turns: HashMap<String, ObservedTurn> = HashMap::new();
    for (path, modified, len) in paths {
        if !cache.get(&path).is_some_and(|log| log.len == len && log.modified == modified) {
            let previous = cache.remove(&path);
            let Ok(file) = File::open(&path) else { continue };
            if let Some(log) = read_log(BufReader::new(file), len, modified, previous) {
                cache.insert(path.clone(), log);
            }
        }
        let Some(log) = cache.get(&path) else { continue };
        let Some((status, event_at)) = log.turn else { continue };
        if status != AgentStatus::Working && event_at < cutoff_ms { continue; }
        let activity_at = chrono::DateTime::<chrono::Utc>::from(log.modified).timestamp_millis().max(event_at);
        if turns.get(&log.session).is_none_or(|turn| event_at > turn.event_at) {
            turns.insert(log.session.clone(), ObservedTurn {
                session: log.session.clone(),
                project: log.project.clone(),
                status,
                event_at,
                activity_at,
            });
        }
    }
    turns.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const META: &str = "{\"timestamp\":\"2026-11-20T18:33:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"00000000-0000-0000-0000-000000000000\",\"cwd\":\"/tmp/Duck assistant\"}}\n";
    const STARTED: &str = "{\"timestamp\":\"2026-11-20T18:33:01Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n";
    const COMPLETED: &str = "{\"timestamp\":\"2026-11-20T18:33:02Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\",\"last_assistant_message\":\"texto de prueba\"}}\n";

    fn parse(content: &str, previous: Option<CachedLog>) -> CachedLog {
        read_log(Cursor::new(content.as_bytes()), content.len() as u64, SystemTime::now(), previous).unwrap()
    }

    #[test]
    fn detects_a_turn_in_an_existing_chat_and_stops_at_completion() {
        let mut content = format!("{META}{STARTED}");
        let working = parse(&content, None);
        assert_eq!(working.project, "Duck assistant");
        assert!(matches!(working.turn, Some((AgentStatus::Working, _))));
        content.push_str(COMPLETED);
        assert!(matches!(parse(&content, Some(working)).turn, Some((AgentStatus::Finished, _))));
    }

    #[test]
    fn retries_a_partially_written_record_and_ignores_message_contents() {
        let mut content = format!("{META}{STARTED}");
        content.push_str("{\"type\":\"response_item\",\"payload\":{\"type\":\"task_complete\"}}\n");
        content.push_str(&COMPLETED[..40]);
        let partial = parse(&content, None);
        assert!(matches!(partial.turn, Some((AgentStatus::Working, _))));
        content.push_str(&COMPLETED[40..]);
        content.push_str("{\"timestamp\":\"2026-11-20T18:33:03Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"turn_aborted\"}}\n");
        assert!(matches!(parse(&content, Some(partial)).turn, Some((AgentStatus::Interrupted, _))));
    }
}
