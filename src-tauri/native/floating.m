#import <AppKit/AppKit.h>

static __weak NSWindow *duckWindow;
static BOOL keepDuckInFront = NO;
static BOOL refreshScheduled = NO;

static NSScreen *pointerScreen(void) {
    NSPoint pointer = NSEvent.mouseLocation;
    for (NSScreen *screen in NSScreen.screens) {
        if (NSPointInRect(pointer, screen.frame)) return screen;
    }
    return NSScreen.mainScreen;
}

static void revealDuck(BOOL recenter) {
    NSWindow *window = duckWindow;
    if (!window || !keepDuckInFront) return;
    NSScreen *screen = recenter ? pointerScreen() : nil;
    if (screen) {
        NSRect work = screen.visibleFrame;
        NSSize size = window.frame.size;
        CGFloat x = MAX(NSMinX(work), NSMaxX(work) - size.width - 20);
        CGFloat y = MIN(NSMinY(work) + 20, MAX(NSMinY(work), NSMaxY(work) - size.height));
        [window setFrameOrigin:NSMakePoint(x, y)];
    }
    [window orderFrontRegardless];
}

static void refreshAfterWorkspaceChange(void) {
    if (!keepDuckInFront) return;
    revealDuck(NO);
    if (refreshScheduled) return;
    refreshScheduled = YES;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 280 * NSEC_PER_MSEC), dispatch_get_main_queue(), ^{
        refreshScheduled = NO;
        revealDuck(NO);
    });
}

static void refreshAfterDisplayChange(void) {
    NSWindow *window = duckWindow;
    if (!window || !keepDuckInFront) return;
    for (NSScreen *screen in NSScreen.screens) {
        if (NSIntersectsRect(screen.frame, window.frame)) {
            refreshAfterWorkspaceChange();
            return;
        }
    }
    revealDuck(YES);
}

void duck_join_workspaces(void *nativeWindow) {
    NSWindow *window = (__bridge NSWindow *)nativeWindow;
    duckWindow = window;
    NSWindowCollectionBehavior behavior = window.collectionBehavior;
    behavior &= ~(NSWindowCollectionBehaviorPrimary |
                  NSWindowCollectionBehaviorAuxiliary |
                  NSWindowCollectionBehaviorMoveToActiveSpace |
                  NSWindowCollectionBehaviorFullScreenPrimary |
                  NSWindowCollectionBehaviorFullScreenNone |
                  NSWindowCollectionBehaviorParticipatesInCycle);
    behavior |= NSWindowCollectionBehaviorCanJoinAllSpaces |
                NSWindowCollectionBehaviorCanJoinAllApplications |
                NSWindowCollectionBehaviorFullScreenAuxiliary |
                NSWindowCollectionBehaviorIgnoresCycle;
    window.collectionBehavior = behavior;
    window.hidesOnDeactivate = NO;
    window.level = NSStatusWindowLevel;

    static dispatch_once_t observed;
    dispatch_once(&observed, ^{
        NSWorkspace *workspace = NSWorkspace.sharedWorkspace;
        for (NSNotificationName name in @[NSWorkspaceActiveSpaceDidChangeNotification, NSWorkspaceDidActivateApplicationNotification]) {
            [workspace.notificationCenter addObserverForName:name object:nil queue:NSOperationQueue.mainQueue
                                                 usingBlock:^(__unused NSNotification *notice) { refreshAfterWorkspaceChange(); }];
        }
        [NSNotificationCenter.defaultCenter addObserverForName:NSApplicationDidChangeScreenParametersNotification
                                                       object:nil queue:NSOperationQueue.mainQueue
                                                   usingBlock:^(__unused NSNotification *notice) { refreshAfterDisplayChange(); }];
    });
}

void duck_set_front(int visible, int recenter) {
    keepDuckInFront = visible != 0;
    if (keepDuckInFront) revealDuck(recenter != 0);
}

void duck_resize_frame(double dx, double dy, double width, double height) {
    NSWindow *window = duckWindow;
    NSRect frame = window.frame;
    // Rust measures offsets from the top left. AppKit measures frames from the bottom left.
    [window setFrame:NSMakeRect(NSMinX(frame) + dx, NSMaxY(frame) - dy - height, width, height)
             display:YES];
}

void duck_set_chat_focus(int open, int focus) {
    NSWindow *window = duckWindow;
    if (![window isKindOfClass:NSPanel.class]) return;
    NSPanel *panel = (NSPanel *)window;
    panel.becomesKeyOnlyIfNeeded = open == 0;
    if (focus && open) {
        [panel orderFrontRegardless];
        [panel makeKeyWindow];
    } else if (!open && panel.isKeyWindow) {
        [panel resignKeyWindow];
    }
}
