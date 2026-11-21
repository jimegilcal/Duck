#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>
#import <dispatch/dispatch.h>
#include <stdlib.h>
#include <string.h>

static char *reply(NSDictionary *result) {
    NSData *data = [NSJSONSerialization dataWithJSONObject:result options:0 error:NULL];
    NSString *json = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
    return strdup(json.UTF8String ?: "{\"error\":\"Calendario no disponible.\"}");
}

static char *fail(NSString *message) {
    return reply(@{ @"error": message });
}

static BOOL hasFullAccess(void) {
    EKAuthorizationStatus status = [EKEventStore authorizationStatusForEntityType:EKEntityTypeEvent];
    if (@available(macOS 14.0, *)) return status == EKAuthorizationStatusFullAccess;
    return status == EKAuthorizationStatusAuthorized;
}

static BOOL canUseCalendar(EKCalendar *calendar) {
    return calendar && calendar.allowsContentModifications && calendar.source.sourceType == EKSourceTypeCalDAV;
}

static NSDictionary *describe(EKCalendar *calendar) {
    return @{ @"id": calendar.calendarIdentifier,
              @"title": calendar.title,
              @"account": calendar.source.title ?: @"Cuenta de calendario" };
}

char *duck_list_calendars(void) {
    @autoreleasepool {
        EKAuthorizationStatus status = [EKEventStore authorizationStatusForEntityType:EKEntityTypeEvent];
        if (!hasFullAccess() && status != EKAuthorizationStatusDenied && status != EKAuthorizationStatusRestricted) {
            dispatch_semaphore_t finished = dispatch_semaphore_create(0);
            dispatch_async(dispatch_get_main_queue(), ^{
                EKEventStore *store = [[EKEventStore alloc] init];
                void (^completion)(BOOL, NSError *) = ^(BOOL granted __unused, NSError *error __unused) {
                    dispatch_semaphore_signal(finished);
                    (void)store; // Keep the store alive until the response.
                };
                if (@available(macOS 14.0, *)) {
                    [store requestFullAccessToEventsWithCompletion:completion];
                } else {
                    [store requestAccessToEntityType:EKEntityTypeEvent completion:completion];
                }
            });
            if (dispatch_semaphore_wait(finished, dispatch_time(DISPATCH_TIME_NOW, 120 * NSEC_PER_SEC)) != 0) {
                return fail(@"No se recibió respuesta al permiso de Calendario.");
            }
        }
        if (!hasFullAccess()) return fail(@"Permite a Duck acceder a Calendario en Ajustes del Sistema → Privacidad y seguridad → Calendarios.");
        EKEventStore *store = [[EKEventStore alloc] init];
        NSMutableArray *choices = [NSMutableArray array];
        for (EKCalendar *calendar in [store calendarsForEntityType:EKEntityTypeEvent]) {
            if (canUseCalendar(calendar)) [choices addObject:describe(calendar)];
        }
        return reply(@{ @"value": choices });
    }
}

char *duck_calendar_detail(const char *identifier) {
    @autoreleasepool {
        if (!hasFullAccess()) return reply(@{ @"value": NSNull.null });
        EKEventStore *store = [[EKEventStore alloc] init];
        EKCalendar *calendar = [store calendarWithIdentifier:[NSString stringWithUTF8String:identifier]];
        return reply(@{ @"value": canUseCalendar(calendar) ? (id)describe(calendar) : NSNull.null });
    }
}

char *duck_commit_event(const char *identifier, const char *title, double start, double end) {
    @autoreleasepool {
        if (!hasFullAccess()) return fail(@"El permiso de Calendario ya no está activo. Elige de nuevo el calendario.");
        EKEventStore *store = [[EKEventStore alloc] init];
        EKCalendar *calendar = [store calendarWithIdentifier:[NSString stringWithUTF8String:identifier]];
        if (!canUseCalendar(calendar)) return fail(@"El calendario elegido ya no está disponible para escribir.");
        EKEvent *event = [EKEvent eventWithEventStore:store];
        event.calendar = calendar;
        event.title = [NSString stringWithUTF8String:title];
        event.startDate = [NSDate dateWithTimeIntervalSince1970:start];
        event.endDate = [NSDate dateWithTimeIntervalSince1970:end];
        NSError *error = nil;
        if (![store saveEvent:event span:EKSpanThisEvent commit:YES error:&error]) {
            return fail(error.localizedDescription ?: @"macOS no pudo guardar el evento.");
        }
        return reply(@{ @"value": describe(calendar) });
    }
}

void duck_free_calendar_reply(char *result) { free(result); }
