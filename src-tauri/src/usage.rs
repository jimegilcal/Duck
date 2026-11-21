//! Token and activity totals for Claude Code and Codex, read from the session logs they
//! already keep on this Mac. Only numbers and timestamps are read: the structs below do not
//! declare any field that holds message text, so conversation content is never deserialized.
use chrono::{Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::BufRead,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime},
};
use tauri::{async_runtime, AppHandle, Manager};

use crate::desktop::DesktopState;

const WINDOW_DAYS: i64 = 31;
/// A pause longer than this is not counted as working time.
const MAX_GAP_MS: i64 = 5 * 60_000;
const MAX_DEPTH: u8 = 7;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageRow {
    date: String,
    provider: &'static str,
    project: String,
    /// Model that produced the tokens. Empty on the rows that only carry working time.
    model: String,
    /// New input tokens (for Claude Code this includes tokens written to the cache).
    input: u64,
    output: u64,
    /// Tokens served from the prompt cache. Reported apart so they do not inflate the totals.
    cache: u64,
    active_minutes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReport {
    rows: Vec<UsageRow>,
    generated_at: i64,
    files: usize,
    window_days: i64,
}

#[derive(Clone, Default)]
struct Resp { key: (String, String), ts: i64, project: String, model: String, input: u64, output: u64, cache: u64 }
#[derive(Clone)]
struct Delta { ts: i64, project: String, model: String, input: u64, output: u64, cache: u64 }
#[derive(Clone, Default)]
struct Parsed { project: String, stamps: Vec<i64>, responses: Vec<Resp>, deltas: Vec<Delta> }
struct Cached { len: u64, modified: SystemTime, parsed: Parsed }

// Only these fields are ever read. There is deliberately no `content`, `text` or `prompt`.
#[derive(Deserialize)]
struct Head { timestamp: Option<String>, cwd: Option<String> }
#[derive(Deserialize)]
struct ClaudeLine { #[serde(rename = "requestId")] request_id: Option<String>, message: Option<ClaudeMsg> }
#[derive(Deserialize)]
struct ClaudeMsg { id: Option<String>, model: Option<String>, usage: Option<ClaudeUsage> }
#[derive(Deserialize)]
struct ClaudeUsage {
    input_tokens: Option<u64>, output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>, cache_read_input_tokens: Option<u64>,
}
#[derive(Deserialize)]
struct CodexLine { payload: Option<CodexPayload> }
#[derive(Deserialize)]
struct CodexPayload { #[serde(rename = "type")] kind: Option<String>, info: Option<CodexInfo>, cwd: Option<String>, model: Option<String> }
#[derive(Deserialize)]
struct CodexInfo { total_token_usage: Option<CodexTotal> }
#[derive(Deserialize)]
struct CodexTotal { input_tokens: Option<u64>, cached_input_tokens: Option<u64>, output_tokens: Option<u64> }

pub(crate) fn parse_ms(text: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(text).ok().map(|date| date.timestamp_millis())
}

pub(crate) fn project_name(cwd: &str) -> String {
    let name = Path::new(cwd).file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.is_empty() { "Sin proyecto".into() } else { name.chars().take(80).collect() }
}

fn parse_reader<R: BufRead>(mut reader: R, codex: bool, name: &str) -> Parsed {
    let mut parsed = Parsed::default();
    let (mut prev_input, mut prev_cached, mut prev_output) = (0u64, 0u64, 0u64);
    let mut current_model = String::new();
    let mut buffer = Vec::new();
    let mut number = 0usize;
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer) { Ok(0) | Err(_) => break, Ok(_) => {} }
        number += 1;
        let line = String::from_utf8_lossy(&buffer);
        let Ok(head) = serde_json::from_str::<Head>(&line) else { continue };
        let ts = head.timestamp.as_deref().and_then(parse_ms);
        if let Some(ts) = ts { parsed.stamps.push(ts); }
        if parsed.project.is_empty() {
            if let Some(cwd) = head.cwd.as_deref() { parsed.project = project_name(cwd); }
        }

        if !codex {
            if !line.contains(r#""type":"assistant""#) { continue; }
            let Some(ts) = ts else { continue };
            let Ok(entry) = serde_json::from_str::<ClaudeLine>(&line) else { continue };
            let Some(message) = entry.message else { continue };
            let Some(usage) = message.usage else { continue };
            let (input, output) = (usage.input_tokens.unwrap_or(0), usage.output_tokens.unwrap_or(0));
            let (created, read) = (usage.cache_creation_input_tokens.unwrap_or(0), usage.cache_read_input_tokens.unwrap_or(0));
            if input + output + created + read == 0 { continue; }
            // Each response is written several times while it streams, and resumed sessions copy
            // earlier ones into a new file. Without both ids it cannot be matched, so keep it unique.
            let key = match (message.id, entry.request_id) {
                (Some(id), Some(request)) => (id, request),
                _ => (format!("~{name}:{number}"), String::new()),
            };
            let project = head.cwd.as_deref().map(project_name).unwrap_or_else(|| parsed.project.clone());
            let model = message.model.filter(|m| !m.starts_with('<')).unwrap_or_default();
            parsed.responses.push(Resp { key, ts, project, model, input: input + created, output, cache: read });
        } else {
            if line.contains(r#""session_meta""#) && parsed.project.is_empty() {
                if let Ok(CodexLine { payload: Some(CodexPayload { cwd: Some(cwd), .. }) }) = serde_json::from_str(&line) {
                    parsed.project = project_name(&cwd);
                }
            }
            // Each turn announces its model; later token counts belong to the model announced last.
            if line.contains(r#""turn_context""#) {
                if let Ok(CodexLine { payload: Some(CodexPayload { model: Some(model), .. }) }) = serde_json::from_str(&line) {
                    current_model = model;
                }
            }
            if !line.contains(r#""token_count""#) { continue; }
            let Some(ts) = ts else { continue };
            let Ok(CodexLine { payload: Some(payload) }) = serde_json::from_str::<CodexLine>(&line) else { continue };
            if payload.kind.as_deref() != Some("token_count") { continue; }
            let Some(total) = payload.info.and_then(|info| info.total_token_usage) else { continue };
            let (input, cached, output) = (total.input_tokens.unwrap_or(0), total.cached_input_tokens.unwrap_or(0), total.output_tokens.unwrap_or(0));
            // Totals are cumulative for the session, so a turn is the difference from the last one.
            let (delta_input, delta_cached, delta_output) = if input >= prev_input && cached >= prev_cached && output >= prev_output {
                (input - prev_input, cached - prev_cached, output - prev_output)
            } else {
                (input, cached, output)
            };
            (prev_input, prev_cached, prev_output) = (input, cached, output);
            if delta_input + delta_output > 0 {
                // Codex counts cached tokens inside its input total; split them out.
                parsed.deltas.push(Delta {
                    ts, project: parsed.project.clone(), model: current_model.clone(),
                    input: delta_input.saturating_sub(delta_cached), output: delta_output, cache: delta_cached,
                });
            }
        }
    }
    parsed
}

fn day(ts: i64) -> Option<String> {
    Local.timestamp_millis_opt(ts).single().map(|date| date.format("%Y-%m-%d").to_string())
}

#[derive(Default)]
struct Acc { input: u64, output: u64, cache: u64, active_ms: i64 }

fn aggregate(items: &[(&'static str, &Parsed)], cutoff: i64) -> Vec<UsageRow> {
    let mut totals: BTreeMap<(String, &'static str, String, String), Acc> = BTreeMap::new();

    let mut responses: HashMap<(String, String), &Resp> = HashMap::new();
    for (_, parsed) in items.iter().filter(|(provider, _)| *provider == "claude-code") {
        for response in &parsed.responses {
            let better = responses.get(&response.key).is_none_or(|kept| {
                (response.output, response.input + response.cache) > (kept.output, kept.input + kept.cache)
            });
            if better { responses.insert(response.key.clone(), response); }
        }
    }
    for response in responses.values().filter(|r| r.ts >= cutoff) {
        let Some(date) = day(response.ts) else { continue };
        let acc = totals.entry((date, "claude-code", response.project.clone(), response.model.clone())).or_default();
        acc.input += response.input; acc.output += response.output; acc.cache += response.cache;
    }
    for (_, parsed) in items.iter().filter(|(provider, _)| *provider == "codex") {
        for delta in parsed.deltas.iter().filter(|d| d.ts >= cutoff) {
            let Some(date) = day(delta.ts) else { continue };
            let acc = totals.entry((date, "codex", delta.project.clone(), delta.model.clone())).or_default();
            acc.input += delta.input; acc.output += delta.output; acc.cache += delta.cache;
        }
    }

    // Working time: merge every timestamp of a tool+project (so overlapping or copied sessions
    // are not counted twice) and add the short gaps between consecutive events.
    let mut stamps: HashMap<(&'static str, String), Vec<i64>> = HashMap::new();
    for (provider, parsed) in items {
        let project = if parsed.project.is_empty() { "Sin proyecto".to_string() } else { parsed.project.clone() };
        stamps.entry((*provider, project)).or_default().extend(parsed.stamps.iter().copied());
    }
    for ((provider, project), mut list) in stamps {
        list.sort_unstable();
        list.dedup();
        for pair in list.windows(2) {
            let gap = pair[1] - pair[0];
            if gap > MAX_GAP_MS || pair[1] < cutoff { continue; }
            let Some(date) = day(pair[1]) else { continue };
            totals.entry((date, provider, project.clone(), String::new())).or_default().active_ms += gap;
        }
    }

    totals.into_iter().map(|((date, provider, project, model), acc)| UsageRow {
        date, provider, project, model, input: acc.input, output: acc.output, cache: acc.cache,
        active_minutes: (acc.active_ms as f64 / 60_000.0).round() as u64,
    }).filter(|row| row.input + row.output + row.cache + row.active_minutes > 0).collect()
}

pub(crate) fn collect(dir: &Path, depth: u8, cutoff: SystemTime, out: &mut Vec<(PathBuf, SystemTime, u64)>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else { continue };
        if meta.file_type().is_symlink() { continue; }
        if meta.is_dir() {
            if depth < MAX_DEPTH { collect(&path, depth + 1, cutoff, out); }
        } else if meta.is_file() && path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            if let Ok(modified) = meta.modified() {
                if modified >= cutoff { out.push((path, modified, meta.len())); }
            }
        }
    }
}

fn cache() -> &'static Mutex<HashMap<PathBuf, Cached>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Cached>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn compute() -> UsageReport {
    let now = SystemTime::now();
    let cutoff_time = now - Duration::from_secs(WINDOW_DAYS as u64 * 86_400);
    let cutoff_ms = chrono::Utc::now().timestamp_millis() - WINDOW_DAYS * 86_400_000;
    let mut found: Vec<(&'static str, PathBuf, SystemTime, u64)> = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        for (provider, root) in [("claude-code", home.join(".claude/projects")), ("codex", home.join(".codex/sessions"))] {
            let mut files = Vec::new();
            collect(&root, 0, cutoff_time, &mut files);
            found.extend(files.into_iter().map(|(path, modified, len)| (provider, path, modified, len)));
        }
    }

    let mut guard = cache().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.retain(|path, _| found.iter().any(|(_, found_path, _, _)| found_path == path));
    for (provider, path, modified, len) in &found {
        if guard.get(path).is_some_and(|cached| cached.len == *len && cached.modified == *modified) { continue; }
        let Ok(file) = fs::File::open(path) else { continue };
        let reader = std::io::BufReader::with_capacity(1 << 20, file);
        let parsed = parse_reader(reader, *provider == "codex", &path.to_string_lossy());
        guard.insert(path.clone(), Cached { len: *len, modified: *modified, parsed });
    }
    let items: Vec<(&'static str, &Parsed)> = found.iter()
        .filter_map(|(provider, path, _, _)| guard.get(path).map(|cached| (*provider, &cached.parsed)))
        .collect();
    UsageReport {
        rows: aggregate(&items, cutoff_ms),
        generated_at: chrono::Utc::now().timestamp_millis(),
        files: items.len(),
        window_days: WINDOW_DAYS,
    }
}

/// Reads the logs only when the user has switched the feature on in Settings.
#[tauri::command]
pub async fn usage_report(app: AppHandle) -> Result<UsageReport, String> {
    let enabled = app.state::<DesktopState>().lock().map_err(|e| e.to_string())?.settings.usage_tracking;
    if !enabled { return Err("usage_disabled".into()); }
    async_runtime::spawn_blocking(compute).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const T0: &str = "2026-11-20T10:00:00.000Z";
    const T1: &str = "2026-11-20T10:01:00.000Z";
    const T2: &str = "2026-11-20T11:30:00.000Z";

    fn claude_line(ts: &str, output: u64) -> String {
        format!(r#"{{"type":"assistant","timestamp":"{ts}","cwd":"/a/Duck","requestId":"r1","message":{{"id":"m1","content":[{{"type":"text","text":"SECRETO"}}],"usage":{{"input_tokens":2,"output_tokens":{output},"cache_creation_input_tokens":10,"cache_read_input_tokens":100}}}}}}"#)
    }
    fn codex_total(ts: &str, input: u64, cached: u64, output: u64) -> String {
        format!(r#"{{"timestamp":"{ts}","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":{input},"cached_input_tokens":{cached},"output_tokens":{output}}}}}}}}}"#)
    }

    /// Manual check against the logs on this Mac: `cargo test real_logs -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_logs_smoke() {
        let start = std::time::Instant::now();
        let first = compute();
        let cold = start.elapsed();
        let start = std::time::Instant::now();
        let second = compute();
        let warm = start.elapsed();
        let mut by: BTreeMap<&str, (u64, u64, u64, u64)> = BTreeMap::new();
        for r in &first.rows {
            let e = by.entry(r.provider).or_default();
            e.0 += r.input; e.1 += r.output; e.2 += r.cache; e.3 += r.active_minutes;
        }
        println!("archivos: {} | filas: {} | primera lectura {:?} | segunda {:?}", first.files, first.rows.len(), cold, warm);
        for (p, (i, o, c, m)) in &by { println!("{p}: entrada {i} salida {o} caché {c} minutos {m}"); }
        let days: std::collections::BTreeSet<_> = first.rows.iter().map(|r| r.date.clone()).collect();
        println!("días con datos: {:?}", days.iter().rev().take(5).collect::<Vec<_>>());
        assert_eq!(first.rows.len(), second.rows.len());
    }

    #[test]
    fn claude_streamed_response_is_counted_once_with_final_output() {
        let text = [claude_line(T0, 5), claude_line(T1, 169)].join("\n");
        let parsed = parse_reader(Cursor::new(text), false, "a");
        let rows = aggregate(&[("claude-code", &parsed)], 0);
        let tokens: u64 = rows.iter().map(|r| r.input).sum();
        assert_eq!((tokens, rows.iter().map(|r| r.output).sum::<u64>(), rows.iter().map(|r| r.cache).sum::<u64>()), (12, 169, 100));
        assert_eq!(rows[0].project, "Duck");
    }

    #[test]
    fn tokens_are_split_by_model_and_time_has_no_model() {
        let claude = parse_reader(Cursor::new(claude_line(T0, 40).replace(r#""id":"m1""#, r#""id":"m1","model":"claude-opus-5-5""#)), false, "a");
        let rows = aggregate(&[("claude-code", &claude)], 0);
        assert!(rows.iter().any(|r| r.model == "claude-opus-5-5" && r.output == 40));
        assert!(rows.iter().all(|r| r.model.is_empty() || r.active_minutes == 0));

        let turn = |ts: &str, model: &str| format!(r#"{{"timestamp":"{ts}","type":"turn_context","payload":{{"model":"{model}"}}}}"#);
        let text = [turn(T0, "gpt-a"), codex_total(T0, 1000, 0, 100), turn(T1, "gpt-b"), codex_total(T1, 1600, 0, 150)].join("\n");
        let codex = parse_reader(Cursor::new(text), true, "c");
        let rows = aggregate(&[("codex", &codex)], 0);
        let of = |m: &str| rows.iter().filter(|r| r.model == m).map(|r| (r.input, r.output)).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        assert_eq!((of("gpt-a"), of("gpt-b")), ((1000, 100), (600, 50)));
    }

    #[test]
    fn same_response_in_two_files_is_counted_once() {
        let a = parse_reader(Cursor::new(claude_line(T0, 50)), false, "a");
        let b = parse_reader(Cursor::new(claude_line(T0, 50)), false, "b");
        let rows = aggregate(&[("claude-code", &a), ("claude-code", &b)], 0);
        assert_eq!(rows.iter().map(|r| r.output).sum::<u64>(), 50);
    }

    #[test]
    fn codex_turns_are_differences_and_repeats_add_nothing() {
        let text = [
            r#"{"timestamp":"2026-11-20T10:00:00.000Z","type":"session_meta","payload":{"cwd":"/x/Proyecto"}}"#.to_string(),
            codex_total(T0, 1000, 400, 50), codex_total(T0, 1000, 400, 50), codex_total(T1, 1500, 700, 80),
        ].join("\n");
        let parsed = parse_reader(Cursor::new(text), true, "c");
        let rows = aggregate(&[("codex", &parsed)], 0);
        assert_eq!((rows.iter().map(|r| r.input).sum::<u64>(), rows.iter().map(|r| r.output).sum::<u64>(), rows.iter().map(|r| r.cache).sum::<u64>()), (800, 80, 700));
        assert_eq!(rows[0].project, "Proyecto");
    }

    #[test]
    fn active_time_skips_long_pauses_and_ignores_text() {
        let text = [claude_line(T0, 5), claude_line(T1, 6), claude_line(T2, 7)].join("\n");
        let parsed = parse_reader(Cursor::new(text), false, "a");
        let minutes: u64 = aggregate(&[("claude-code", &parsed)], 0).iter().map(|r| r.active_minutes).sum();
        assert_eq!(minutes, 1); // 10:00→10:01 counts; the 89-minute pause does not
        assert!(!format!("{:?}", aggregate(&[("claude-code", &parsed)], 0)).contains("SECRETO"));
    }
}
