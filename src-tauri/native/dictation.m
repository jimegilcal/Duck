#import <AVFoundation/AVFoundation.h>
#import <Speech/Speech.h>
#include <stdint.h>

typedef NS_ENUM(int32_t, DuckDictationEvent) {
    DuckDictationListening = 0,
    DuckDictationPartial = 1,
    DuckDictationFinished = 2,
    DuckDictationFailed = 3,
    DuckDictationCancelled = 4,
    DuckDictationDownloading = 5,
    DuckDictationTranscribing = 6,
};

typedef void (*DuckDictationCallback)(void *, int32_t, const char *);

extern uint8_t duck_modern_dictation_available(void);
extern void duck_start_modern_dictation(void *, DuckDictationCallback);
extern void duck_stop_modern_dictation(void);
extern void duck_cancel_modern_dictation(void);

@interface DuckDictation : NSObject
@property(nonatomic, copy) NSString *identifier;
@property(nonatomic) uint8_t window;
@property(nonatomic) void *context;
@property(nonatomic) DuckDictationCallback callback;
@property(nonatomic, strong) SFSpeechRecognizer *recognizer;
@property(nonatomic, strong) AVAudioEngine *engine;
@property(nonatomic, strong) SFSpeechAudioBufferRecognitionRequest *request;
@property(nonatomic, strong) SFSpeechRecognitionTask *task;
@property(nonatomic, copy) NSString *transcript;
@property(nonatomic) BOOL hasTap;
@property(nonatomic) BOOL stopping;
@property(nonatomic) BOOL modern;
@property(nonatomic) BOOL modernStarted;
@end

@implementation DuckDictation
@end

// The session and all UI callbacks are confined to the main queue.
static DuckDictation *activeDictation;

static void emit(DuckDictation *session, DuckDictationEvent event, NSString *text) {
    session.callback(session.context, event, text.UTF8String);
}

static void stopInput(DuckDictation *session) {
    if (!session.hasTap) return;
    [session.engine stop];
    [session.engine.inputNode removeTapOnBus:0];
    session.hasTap = NO;
    [session.request endAudio];
}

static void complete(DuckDictation *session, DuckDictationEvent event, NSString *text) {
    if (!session || activeDictation != session) return;
    activeDictation = nil;
    if (session.modern) duck_cancel_modern_dictation();
    stopInput(session);
    [session.task cancel];
    session.task = nil;
    emit(session, event, text);
}

static void finishTranscript(DuckDictation *session) {
    if (session.transcript.length) {
        complete(session, DuckDictationFinished, session.transcript);
    } else {
        complete(session, DuckDictationFailed, @"No he detectado voz. Vuelve a pulsar el micrófono y habla.");
    }
}

static void stopDictation(DuckDictation *session) {
    if (!session || session.stopping) return;
    if (session.modern) {
        session.stopping = YES;
        if (session.modernStarted) duck_stop_modern_dictation();
        else complete(session, DuckDictationCancelled, nil);
        return;
    }
    if (!session.hasTap) {
        complete(session, DuckDictationCancelled, nil);
        return;
    }
    session.stopping = YES;
    stopInput(session);
    emit(session, DuckDictationTranscribing, nil);
    __weak DuckDictation *weakSession = session;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 4 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{
        DuckDictation *session = weakSession;
        if (session && activeDictation == session) finishTranscript(session);
    });
}

static void startCapture(DuckDictation *session) {
    AVAudioEngine *engine = [AVAudioEngine new];
    AVAudioInputNode *input = engine.inputNode;
    AVAudioFormat *format = [input outputFormatForBus:0];
    if (!format.channelCount || format.sampleRate <= 0) {
        complete(session, DuckDictationFailed, @"No hay un micrófono disponible. Comprueba la entrada de sonido de tu Mac.");
        return;
    }
    session.engine = engine;
    SFSpeechAudioBufferRecognitionRequest *request = [SFSpeechAudioBufferRecognitionRequest new];
    request.requiresOnDeviceRecognition = YES;
    request.shouldReportPartialResults = YES;
    request.taskHint = SFSpeechRecognitionTaskHintDictation;
    session.request = request;

    NSError *audioError = nil;
    @try {
        [input installTapOnBus:0 bufferSize:1024 format:format block:^(AVAudioPCMBuffer *buffer, AVAudioTime *when) {
            (void)when;
            [request appendAudioPCMBuffer:buffer];
        }];
        session.hasTap = YES;
        [engine prepare];
        if (![engine startAndReturnError:&audioError]) {
            complete(session, DuckDictationFailed, @"No se pudo iniciar el micrófono. Comprueba la entrada de sonido de tu Mac.");
            return;
        }
    } @catch (NSException *exception) {
        (void)exception;
        complete(session, DuckDictationFailed, @"El micrófono no está disponible. Revisa la entrada de sonido e inténtalo otra vez.");
        return;
    }

    __weak DuckDictation *weakSession = session;
    session.task = [session.recognizer recognitionTaskWithRequest:request resultHandler:^(SFSpeechRecognitionResult *result, NSError *error) {
        DuckDictation *session = weakSession;
        if (!session || activeDictation != session) return;
        NSString *text = result.bestTranscription.formattedString;
        if (text.length) {
            session.transcript = text;
            if (!result.isFinal) emit(session, DuckDictationPartial, text);
        }
        if (result.isFinal || (error && (session.stopping || session.transcript.length))) {
            finishTranscript(session);
        } else if (error) {
            complete(session, DuckDictationFailed, @"El dictado local no está disponible ahora. Comprueba que el español esté configurado en Ajustes del Sistema → Teclado → Dictado.");
        }
    }];
    emit(session, DuckDictationListening, nil);
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 55 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{
        DuckDictation *session = weakSession;
        if (session && activeDictation == session) stopDictation(session);
    });
}

static void checkSpeechPermission(DuckDictation *session, SFSpeechRecognizerAuthorizationStatus status) {
    if (activeDictation != session) return;
    if (status == SFSpeechRecognizerAuthorizationStatusAuthorized) {
        startCapture(session);
    } else {
        complete(session, DuckDictationFailed, @"Permite Reconocimiento de voz para Duck en Ajustes del Sistema → Privacidad y seguridad.");
    }
}

static void modernDictationEvent(void *context, int32_t code, const char *text) {
    DuckDictation *session = (__bridge DuckDictation *)context;
    if (activeDictation != session) return;
    NSString *message = text ? [NSString stringWithUTF8String:text] : nil;
    if (code >= DuckDictationFinished && code <= DuckDictationCancelled) {
        complete(session, code, message);
    } else {
        emit(session, code, message);
    }
}

static void checkMicrophonePermission(DuckDictation *session, BOOL granted) {
    if (activeDictation != session) return;
    if (!granted) {
        complete(session, DuckDictationFailed, @"Permite el micrófono para Duck en Ajustes del Sistema → Privacidad y seguridad → Micrófono.");
        return;
    }
    if (session.modern) {
        session.modernStarted = YES;
        duck_start_modern_dictation((__bridge void *)session, modernDictationEvent);
        return;
    }
    SFSpeechRecognizerAuthorizationStatus status = SFSpeechRecognizer.authorizationStatus;
    if (status == SFSpeechRecognizerAuthorizationStatusNotDetermined) {
        [SFSpeechRecognizer requestAuthorization:^(SFSpeechRecognizerAuthorizationStatus decision) {
            dispatch_async(dispatch_get_main_queue(), ^{ checkSpeechPermission(session, decision); });
        }];
    } else {
        checkSpeechPermission(session, status);
    }
}

void duck_start_dictation(const char *identifier, uint8_t window, void *context, DuckDictationCallback callback) {
    if (activeDictation) {
        callback(context, DuckDictationFailed, "Ya hay un dictado en curso en la otra ventana de Duck.");
        return;
    }
    DuckDictation *session = [DuckDictation new];
    session.identifier = [NSString stringWithUTF8String:identifier];
    session.window = window;
    session.context = context;
    session.callback = callback;
    session.modern = duck_modern_dictation_available();
    activeDictation = session;

    if (!session.modern) {
        for (NSString *locale in @[@"es-ES", @"es-MX"]) {
            SFSpeechRecognizer *recognizer = [[SFSpeechRecognizer alloc] initWithLocale:[NSLocale localeWithLocaleIdentifier:locale]];
            if (recognizer.supportsOnDeviceRecognition) {
                session.recognizer = recognizer;
                recognizer.queue = NSOperationQueue.mainQueue;
                break;
            }
        }
        if (!session.recognizer) {
            complete(session, DuckDictationFailed, @"El dictado local en español no está disponible en este Mac. Revisa los idiomas en Ajustes del Sistema → Teclado → Dictado.");
            return;
        }
    }
    AVAuthorizationStatus status = [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
    if (status == AVAuthorizationStatusNotDetermined) {
        [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio completionHandler:^(BOOL granted) {
            dispatch_async(dispatch_get_main_queue(), ^{ checkMicrophonePermission(session, granted); });
        }];
    } else {
        checkMicrophonePermission(session, status == AVAuthorizationStatusAuthorized);
    }
}

void duck_end_dictation(const char *identifier, uint8_t window, uint8_t cancel) {
    DuckDictation *session = activeDictation;
    if (!session || session.window != window || ![session.identifier isEqualToString:[NSString stringWithUTF8String:identifier]]) return;
    if (cancel) complete(session, DuckDictationCancelled, nil);
    else stopDictation(session);
}

void duck_cancel_window_dictation(uint8_t window) {
    DuckDictation *session = activeDictation;
    if (session && session.window == window) complete(session, DuckDictationCancelled, nil);
}
