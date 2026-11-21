#!/usr/bin/python3
"""Genera la configuración de hooks de Claude Code para Duck, con la ruta de ESTA copia del proyecto.

No modifica nada: imprime el bloque "hooks" para que lo pegues tú en ~/.claude/settings.json
(si ya tienes hooks allí, fusiona cada lista a mano en lugar de reemplazarla).

    python3 scripts/install-claude-hooks.py            # imprime el bloque
    python3 scripts/install-claude-hooks.py --check    # además comprueba que el hook funciona
"""
import json
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parent
hook = root / "duck_agent_hook.py"
template = root / "duck-claude-hooks.template.json"

if '"' in str(hook) or "\n" in str(hook):
    sys.exit("La ruta del proyecto contiene comillas o saltos de línea; muévelo a una carpeta con un nombre sencillo.")
if not hook.is_file() or not template.is_file():
    sys.exit("Faltan scripts/duck_agent_hook.py o scripts/duck-claude-hooks.template.json.")

config = json.loads(template.read_text(encoding="utf-8").replace("__HOOK_PATH__", str(hook).replace("\\", "\\\\")))

if "--check" in sys.argv:
    probe = json.dumps({"hook_event_name": "SessionStart", "session_id": "duck-install-check", "cwd": str(root)})
    result = subprocess.run(["/usr/bin/python3", str(hook), "claude-code"], input=probe, capture_output=True, text=True, timeout=10)
    if result.returncode != 0 or result.stdout.strip() != "{}":
        sys.exit(f"El hook no responde como se espera (código {result.returncode}): {result.stderr.strip()[:300]}")
    # Limpia la sesión de comprobación para que no aparezca en Duck.
    import hashlib, os
    key = hashlib.sha256(b"claude-code:duck-install-check").hexdigest()
    (Path("/tmp") / f"duck-agent-events-{os.getuid()}" / f"{key}.json").unlink(missing_ok=True)
    print("El hook funciona.", file=sys.stderr)

print(json.dumps(config, indent=2, ensure_ascii=False))
print("\nPega el bloque \"hooks\" de arriba en ~/.claude/settings.json y reinicia Claude Code.", file=sys.stderr)
