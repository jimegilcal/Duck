#import <AppKit/AppKit.h>
#include <ctype.h>
#include <libproc.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

enum DuckAgentPresence {
    DuckCodex = 1 << 0,
    DuckClaudeCode = 1 << 1,
};

uint32_t duck_running_agents(void) {
    @autoreleasepool {
        uint32_t result = 0;
        for (NSRunningApplication *app in NSWorkspace.sharedWorkspace.runningApplications) {
            NSString *identifier = app.bundleIdentifier;
            if ([identifier isEqualToString:@"com.openai.codex"]) result |= DuckCodex;
        }

        // Inspect executable metadata without opening any session files.
        int capacity = proc_listallpids(NULL, 0) + 64;
        if (capacity <= 64) return result;
        pid_t *pids = calloc((size_t)capacity, sizeof(*pids));
        if (!pids) return result;
        int count = proc_listallpids(pids, capacity * (int)sizeof(*pids));
        for (int i = 0; i < count && i < capacity; i++) {
            char name[64] = {0};
            if (proc_name(pids[i], name, sizeof(name)) <= 0) continue;
            if (strcmp(name, "codex") == 0) result |= DuckCodex;
            if (strcmp(name, "claude") == 0) result |= DuckClaudeCode;
            if (!(result & DuckClaudeCode) && isdigit((unsigned char)name[0])) {
                // The standalone Claude Code installer names the executable after its version.
                char path[PROC_PIDPATHINFO_MAXSIZE] = {0};
                if (proc_pidpath(pids[i], path, sizeof(path)) > 0 &&
                    strstr(path, "/.local/share/claude/versions/") != NULL) result |= DuckClaudeCode;
            }
            if ((result & (DuckCodex | DuckClaudeCode)) == (DuckCodex | DuckClaudeCode)) break;
        }
        free(pids);
        return result;
    }
}
