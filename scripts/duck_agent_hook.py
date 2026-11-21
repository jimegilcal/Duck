"""Send a lifecycle status to Duck through a private local directory.

Codex only reports the state of a turn. Claude Code also reports a short
summary of its progress: the request, the checklist it keeps, the sub-agents it
launches and a small timeline. Everything stays in a 0700 directory owned by the
current user; shell commands, file contents and tool output are never stored.
"""

import fcntl
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import time

STATES = {
    "SessionStart": "ready",
    "UserPromptSubmit": "working",
    "PermissionRequest": "approval",
    "Notification": "approval",
    "Stop": "finished",
    "Interrupt": "interrupted",
    "SessionEnd": "closed",
}
TOOL_EVENTS = {"PreToolUse", "PostToolUse"}
SOURCES = {"codex", "claude-code"}
AGENT_TOOLS = {"Task", "Agent"}
FILE_TOOLS = {"Edit", "MultiEdit", "Write", "NotebookEdit"}
TODO_STATES = {"pending", "in_progress", "completed"}
MAX_SIGNAL_AGE = 12 * 3600
MAX_FILE_BYTES = 60_000
MAX_TODOS = 30
MAX_AGENTS = 40
MAX_TIMELINE = 25


def text(value, limit):
    """Return a single-line, bounded string, or an empty one."""
    if not isinstance(value, str):
        return ""
    return " ".join(value.split())[:limit]


def load(path, uid):
    try:
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != uid or info.st_size > MAX_FILE_BYTES:
            return None
        data = json.loads(path.read_text(encoding="utf-8"))
        return data if isinstance(data, dict) else None
    except (OSError, ValueError):
        return None


def fresh(source, project, now_ms):
    return {
        "provider": source, "status": "ready", "project": project, "timestamp": now_ms,
        "goal": "", "todos": [], "agents": [], "timeline": [],
    }


def note(state, now_ms, message):
    timeline = state.setdefault("timeline", [])
    timeline.append({"t": now_ms, "text": message})
    del timeline[:-MAX_TIMELINE]


def todos_from(tool_input):
    items = tool_input.get("todos") if isinstance(tool_input, dict) else None
    if not isinstance(items, list):
        return None
    result = []
    for item in items[:MAX_TODOS]:
        if not isinstance(item, dict):
            continue
        title = text(item.get("content"), 110)
        status = item.get("status")
        if title and status in TODO_STATES:
            result.append({"title": title, "status": status, "active": text(item.get("activeForm"), 110)})
    return result


def apply_claude_code(state, event, payload, now_ms):
    """Fold one Claude Code event into the session summary."""
    tool = payload.get("tool_name") if isinstance(payload.get("tool_name"), str) else ""
    tool_input = payload.get("tool_input")

    if event == "SessionStart" and payload.get("source") in (None, "startup", "clear"):
        state.update(goal="", todos=[], agents=[], timeline=[])
        note(state, now_ms, "Sesión iniciada")
    elif event == "UserPromptSubmit":
        goal = text(payload.get("prompt"), 140)
        state.update(goal=goal, todos=[], agents=[], timeline=[])
        note(state, now_ms, "Nueva petición")
    elif event in ("Notification", "PermissionRequest"):
        note(state, now_ms, "Espera tu aprobación")
    elif event == "Stop":
        note(state, now_ms, "Terminó su turno")
    elif event == "SessionEnd":
        note(state, now_ms, "Sesión cerrada")
    elif event == "PreToolUse" and tool in AGENT_TOOLS and isinstance(tool_input, dict):
        agents = state.setdefault("agents", [])
        agents.append({
            "id": text(payload.get("tool_use_id"), 80) or str(now_ms),
            "title": text(tool_input.get("description"), 90) or "Agente",
            "kind": text(tool_input.get("subagent_type"), 40),
            "status": "running", "started": now_ms, "ended": None,
        })
        del agents[:-MAX_AGENTS]
        note(state, now_ms, "Lanzó un agente · " + agents[-1]["title"])
    elif event == "PostToolUse" and tool in AGENT_TOOLS:
        wanted = text(payload.get("tool_use_id"), 80)
        for agent in state.get("agents", []):
            if agent.get("id") == wanted and agent.get("status") == "running":
                agent["status"], agent["ended"] = "done", now_ms
                note(state, now_ms, "Agente terminado · " + str(agent.get("title", "")))
                break
    elif event == "PostToolUse" and tool == "TodoWrite":
        todos = todos_from(tool_input)
        if todos is not None:
            state["todos"] = todos
            done = sum(1 for item in todos if item["status"] == "completed")
            note(state, now_ms, f"Plan actualizado · {done}/{len(todos)} pasos")
    elif event == "PostToolUse" and tool in FILE_TOOLS and isinstance(tool_input, dict):
        name = Path(text(tool_input.get("file_path") or tool_input.get("notebook_path"), 300)).name
        note(state, now_ms, "Editó " + (name[:60] or "un archivo"))
    elif event == "PostToolUse" and tool == "Bash":
        note(state, now_ms, "Ejecutó un comando")


def send(source, payload):
    event = payload.get("hook_event_name")
    session = payload.get("session_id")
    cwd = payload.get("cwd")
    if (not isinstance(event, str) or not (event in STATES or event in TOOL_EVENTS)
            or not isinstance(session, str) or not session
            or not isinstance(cwd, str) or not Path(cwd).is_absolute()):
        return
    if event == "SessionStart" and payload.get("source") == "compact":
        return
    if event in TOOL_EVENTS and source != "claude-code":
        return
    # A pending-input prompt is not a request for permission.
    if event == "Notification":
        kind = payload.get("notification_type")
        message = str(payload.get("message", "")).lower()
        if kind not in (None, "permission_prompt") or (kind is None and "permission" not in message):
            return

    uid = os.getuid()
    folder = Path("/tmp") / f"duck-agent-events-{uid}"
    folder.mkdir(mode=0o700, exist_ok=True)
    metadata = folder.lstat()
    if not stat.S_ISDIR(metadata.st_mode) or metadata.st_uid != uid or metadata.st_mode & 0o077:
        return

    now_ms = time.time_ns() // 1_000_000
    timestamp = now_ms // 1000
    for stale in folder.glob("*.json"):
        if len(stale.stem) != 64 or any(char not in "0123456789abcdef" for char in stale.stem):
            continue
        try:
            info = stale.lstat()
        except FileNotFoundError:
            continue
        if stat.S_ISREG(info.st_mode) and info.st_uid == uid and info.st_mtime < timestamp - MAX_SIGNAL_AGE:
            stale.unlink(missing_ok=True)

    key = hashlib.sha256((source + ":" + session).encode("utf-8")).hexdigest()
    target = folder / (key + ".json")
    project = (Path(cwd).name or "Proyecto local")[:100]

    # Parallel hooks (sub-agents) must not overwrite each other's updates.
    lock_fd = os.open(folder / ".lock", os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    try:
        fcntl.flock(lock_fd, fcntl.LOCK_EX)
        state = load(target, uid) or fresh(source, project, now_ms)
        state.update(provider=source, project=project, timestamp=now_ms, session=key[:12])
        if event in STATES:
            state["status"] = STATES[event]
        elif state.get("status") in ("approval", "ready", "interrupted"):
            state["status"] = "working"  # a tool ran, so the request was granted
        if source == "claude-code":
            apply_claude_code(state, event, payload, now_ms)
        while len(json.dumps(state)) > MAX_FILE_BYTES and state.get("timeline"):
            del state["timeline"][0]
        with tempfile.NamedTemporaryFile(mode="w", dir=folder, encoding="utf-8", delete=False) as out:
            temporary = Path(out.name)
            try:
                json.dump(state, out)
                out.close()
                os.replace(temporary, target)
            finally:
                temporary.unlink(missing_ok=True)
    finally:
        os.close(lock_fd)


if __name__ == "__main__":
    # Hook input is JSON on stdin. Only allowlisted fields are read from it.
    try:
        if len(sys.argv) == 2 and sys.argv[1] in SOURCES:
            payload = json.load(sys.stdin)
            if isinstance(payload, dict):
                send(sys.argv[1], payload)
    except (OSError, ValueError, RecursionError):
        pass
    # A neutral response leaves permission decisions and turn completion to the agent.
    print("{}")
