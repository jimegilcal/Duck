@preconcurrency import AVFoundation
import CoreMedia
import Foundation
import Speech

typealias DuckVoiceCallback = @convention(c) (UnsafeMutableRawPointer?, Int32, UnsafePointer<CChar>?) -> Void

@available(macOS 26.0, *)
@MainActor private var currentDictation: ModernDictation?

@_cdecl("duck_modern_dictation_available")
func modernDictationAvailable() -> UInt8 {
    if #available(macOS 26.0, *), SpeechTranscriber.isAvailable { return 1 }
    return 0
}

@_cdecl("duck_start_modern_dictation")
func startModernDictation(_ context: UnsafeMutableRawPointer?, _ callback: DuckVoiceCallback) {
    if #available(macOS 26.0, *) {
        MainActor.assumeIsolated {
            let session = ModernDictation(context: context, callback: callback)
            currentDictation = session
            session.start()
        }
    }
}

@_cdecl("duck_stop_modern_dictation")
func stopModernDictation() {
    if #available(macOS 26.0, *) {
        MainActor.assumeIsolated { currentDictation?.stop() }
    }
}

@_cdecl("duck_cancel_modern_dictation")
func cancelModernDictation() {
    if #available(macOS 26.0, *) {
        MainActor.assumeIsolated {
            let session = currentDictation
            currentDictation = nil
            session?.dispose()
        }
    }
}

@available(macOS 26.0, *)
@MainActor private final class ModernDictation {
    private let context: UnsafeMutableRawPointer?
    private let callback: DuckVoiceCallback
    private var engine: AVAudioEngine?
    private var analyzer: SpeechAnalyzer?
    private var input: AsyncStream<AnalyzerInput>.Continuation?
    private var preparation: Task<Void, Never>?
    private var results: Task<Void, Never>?
    private var finalization: Task<Void, Never>?
    private var timer: Task<Void, Never>?
    private var stopping = false
    private var finalized = ""
    private var volatile = ""

    init(context: UnsafeMutableRawPointer?, callback: DuckVoiceCallback) {
        self.context = context
        self.callback = callback
    }

    func start() {
        preparation = Task { [weak self] in
            guard let self else { return }
            do {
                try await prepare()
            } catch is CancellationError {
                return
            } catch let issue as DictationError {
                complete(.failed, issue.message)
            } catch {
                complete(.failed, "No se pudo iniciar el dictado local. Comprueba la entrada de sonido e inténtalo otra vez.")
            }
        }
    }

    private func prepare() async throws {
        guard let locale = await SpeechTranscriber.supportedLocale(equivalentTo: Locale(identifier: "es-ES")) else {
            throw DictationError.unsupported
        }
        let transcriber = SpeechTranscriber(locale: locale, preset: .progressiveTranscription)
        do {
            if let request = try await AssetInventory.assetInstallationRequest(supporting: [transcriber]) {
                try Task.checkCancellation()
                emit(.downloading)
                try await request.downloadAndInstall()
            }
        } catch is CancellationError {
            throw CancellationError()
        } catch {
            throw DictationError.assets
        }
        try Task.checkCancellation()

        let engine = AVAudioEngine()
        let microphone = engine.inputNode
        let nativeFormat = microphone.outputFormat(forBus: 0)
        guard nativeFormat.channelCount > 0, nativeFormat.sampleRate > 0 else { throw DictationError.microphone }
        guard
            let format = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [transcriber], considering: nativeFormat),
            let converter = BufferConverter(from: nativeFormat, to: format)
        else { throw DictationError.microphone }
        try Task.checkCancellation()
        let analyzer = SpeechAnalyzer(modules: [transcriber])
        self.analyzer = analyzer
        try await analyzer.prepareToAnalyze(in: format)
        try Task.checkCancellation()
        let (sequence, input) = AsyncStream<AnalyzerInput>.makeStream(bufferingPolicy: .bufferingNewest(64))
        self.input = input
        results = Task { [weak self] in
            do {
                for try await result in transcriber.results {
                    guard let self else { return }
                    receive(result)
                }
            } catch {
                if let self, !Task.isCancelled, !stopping { finishTranscript() }
            }
        }
        try await analyzer.start(inputSequence: sequence)
        try Task.checkCancellation()

        microphone.installTap(onBus: 0, bufferSize: 1024, format: nativeFormat) { buffer, _ in
            if let converted = converter.convert(buffer) {
                input.yield(AnalyzerInput(buffer: converted))
            }
        }
        self.engine = engine
        engine.prepare()
        try engine.start()
        emit(.listening)
        timer = Task { [weak self] in
            try? await Task.sleep(for: .seconds(55))
            if !Task.isCancelled { self?.stop() }
        }
    }

    private func receive(_ result: SpeechTranscriber.Result) {
        let text = String(result.text.characters).trimmingCharacters(in: .whitespacesAndNewlines)
        if result.isFinal {
            finalized = join(finalized, text)
            volatile = ""
        } else {
            volatile = text
        }
        let transcript = join(finalized, volatile)
        if !transcript.isEmpty { emit(.partial, transcript) }
    }

    func stop() {
        guard !stopping else { return }
        guard engine != nil, let analyzer else {
            complete(.cancelled)
            return
        }
        stopping = true
        closeInput()
        emit(.transcribing)
        finalization = Task { [weak self] in
            // Silence can produce no analyzable audio. Keep any partial text in that case.
            try? await analyzer.finalizeAndFinishThroughEndOfInput()
            await self?.results?.value
            if !Task.isCancelled { self?.finishTranscript() }
        }
        timer?.cancel()
        timer = Task { [weak self] in
            try? await Task.sleep(for: .seconds(5))
            if !Task.isCancelled { self?.finishTranscript() }
        }
    }

    private func closeInput() {
        if let engine {
            engine.stop()
            engine.inputNode.removeTap(onBus: 0)
            self.engine = nil
        }
        input?.finish()
        input = nil
    }

    func dispose() {
        closeInput()
        preparation?.cancel()
        results?.cancel()
        finalization?.cancel()
        timer?.cancel()
        if let analyzer { Task { await analyzer.cancelAndFinishNow() } }
    }

    private func finishTranscript() {
        let transcript = join(finalized, volatile)
        if transcript.isEmpty {
            complete(.failed, "No he detectado voz. Vuelve a pulsar el micrófono y habla.")
        } else {
            complete(.finished, transcript)
        }
    }

    private func complete(_ event: VoiceEvent, _ text: String = "") {
        guard currentDictation === self else { return }
        currentDictation = nil
        dispose()
        emit(event, text)
    }

    private func emit(_ event: VoiceEvent, _ text: String = "") {
        text.withCString { callback(context, event.rawValue, $0) }
    }

    private func join(_ left: String, _ right: String) -> String {
        if left.isEmpty { return right }
        if right.isEmpty { return left }
        return left + " " + right
    }
}

private enum VoiceEvent: Int32 {
    case listening = 0, partial, finished, failed, cancelled, downloading, transcribing
}

private enum DictationError: Error {
    case unsupported, assets, microphone

    var message: String {
        switch self {
        case .unsupported: "El dictado local en español no está disponible en este Mac."
        case .assets: "No se pudo preparar el modelo de español de Apple. Conecta el Mac a Internet e inténtalo otra vez."
        case .microphone: "No hay un micrófono disponible. Comprueba la entrada de sonido de tu Mac."
        }
    }
}

// The audio tap serializes conversion calls, and each result owns its own buffer.
private final class BufferConverter: @unchecked Sendable {
    private let converter: AVAudioConverter
    private let format: AVAudioFormat

    init?(from input: AVAudioFormat, to format: AVAudioFormat) {
        guard let converter = AVAudioConverter(from: input, to: format) else { return nil }
        self.converter = converter
        self.format = format
    }

    func convert(_ buffer: AVAudioPCMBuffer) -> AVAudioPCMBuffer? {
        let capacity = ceil(Double(buffer.frameLength) * format.sampleRate / buffer.format.sampleRate) + 32
        guard let output = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(capacity)) else { return nil }
        let source = ConverterInput(buffer)
        var error: NSError?
        let status = converter.convert(to: output, error: &error) { _, status in
            if let buffer = source.next() {
                status.pointee = .haveData
                return buffer
            }
            status.pointee = .noDataNow
            return nil
        }
        return status == .error || output.frameLength == 0 ? nil : output
    }
}

// AVAudioConverter invokes its input block synchronously during convert(to:error:withInputFrom:).
private final class ConverterInput: @unchecked Sendable {
    private var buffer: AVAudioPCMBuffer?

    init(_ buffer: AVAudioPCMBuffer) { self.buffer = buffer }

    func next() -> AVAudioPCMBuffer? {
        defer { buffer = nil }
        return buffer
    }
}
