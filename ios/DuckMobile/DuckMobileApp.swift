import EventKit
import SceneKit
import SwiftUI

@main
struct DuckMobileApp: App {
    var body: some Scene {
        WindowGroup { CompanionView() }
    }
}

private struct Note: Codable, Identifiable {
    let id: UUID
    let text: String
    let created: Date
}

private struct CompanionView: View {
    @AppStorage("duck.mobile.notes.v1") private var storedNotes = Data()
    @AppStorage("duck.mobile.calendar") private var calendarID = ""
    @State private var input = ""
    @State private var response = ""
    @State private var showEvent = false
    @State private var eventTitle = ""
    @State private var eventStart = Date().addingTimeInterval(3600)
    @State private var scene = makeDuckScene()

    private var notes: [Note] {
        (try? JSONDecoder().decode([Note].self, from: storedNotes)) ?? []
    }

    var body: some View {
        TabView {
            NavigationStack {
                ScrollView {
                    VStack(spacing: 20) {
                        SceneView(scene: scene, options: [])
                            .frame(height: 220)
                            .accessibilityLabel("Duck, tu patito")
                        Text("¿Qué hacemos hoy?").font(.title2.bold())
                        TextField("Anota una idea o escribe «evento: ...»", text: $input, axis: .vertical)
                            .textFieldStyle(.roundedBorder)
                            .submitLabel(.send)
                            .onSubmit(send)
                        HStack {
                            Button("Enviar", action: send)
                                .buttonStyle(.borderedProminent)
                                .disabled(input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                            Button("Nuevo evento") { prepareEvent(title: "") }
                                .buttonStyle(.bordered)
                        }
                        if !response.isEmpty { Text(response).foregroundStyle(.secondary) }
                    }
                    .padding()
                }
                .navigationTitle("Duck")
                .sheet(isPresented: $showEvent) {
                    EventConfirmation(title: eventTitle, start: eventStart, calendarID: $calendarID) { title in
                        response = "Guardado en \(title)."
                        showEvent = false
                    }
                }
            }
            .tabItem { Label("Duck", systemImage: "bubble.left.fill") }
            NavigationStack {
                List {
                    ForEach(notes) { note in
                        VStack(alignment: .leading, spacing: 5) {
                            Text(note.text)
                            Text(note.created, format: .dateTime.day().month().hour().minute())
                                .font(.caption).foregroundStyle(.secondary)
                        }
                    }
                    .onDelete { offsets in
                        saveNotes(notes.enumerated().filter { !offsets.contains($0.offset) }.map(\.element))
                    }
                }
                .overlay {
                    if notes.isEmpty {
                        ContentUnavailableView("Todavía no hay notas", systemImage: "note.text",
                            description: Text("Escribe «anota ...» en el chat de Duck."))
                    }
                }
                .navigationTitle("Notas en este iPhone")
            }
            .tabItem { Label("Notas", systemImage: "note.text") }
        }
        .tint(Color(red: 0.34, green: 0.48, blue: 0.25))
    }

    private func send() {
        let text = input.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = text.lowercased()
        if let prefix = ["nota:", "anota ", "apunta "].first(where: { lower.hasPrefix($0) }) {
            let body = String(text.dropFirst(prefix.count)).trimmingCharacters(in: .whitespacesAndNewlines)
            guard !body.isEmpty else { response = "¿Qué quieres anotar?"; return }
            saveNotes([Note(id: UUID(), text: body, created: Date())] + notes)
            response = "He guardado tu nota en este iPhone."
            input = ""
        } else if let prefix = ["evento:", "agenda ", "agéndame ", "cita:"].first(where: { lower.hasPrefix($0) }) {
            prepareEvent(title: String(text.dropFirst(prefix.count)).trimmingCharacters(in: .whitespacesAndNewlines))
            input = ""
        } else if !text.isEmpty {
            response = "Puedes escribir «anota comprar café» o «evento: reunión». Para un evento podrás elegir la fecha y la hora antes de guardar."
        }
    }

    private func prepareEvent(title: String) {
        eventTitle = title
        eventStart = Date().addingTimeInterval(3600)
        showEvent = true
    }

    private func saveNotes(_ value: [Note]) {
        if let bytes = try? JSONEncoder().encode(value) { storedNotes = bytes }
    }
}

private struct EventConfirmation: View {
    @Environment(\.dismiss) private var dismiss
    @State var title: String
    @State var start: Date
    @Binding var calendarID: String
    let onSave: (String) -> Void
    @State private var store = EKEventStore()
    @State private var calendars: [EKCalendar] = []
    @State private var duration = 30
    @State private var feedback = ""
    @State private var working = false

    var body: some View {
        NavigationStack {
            Form {
                TextField("Título", text: $title)
                DatePicker("Inicio", selection: $start)
                Stepper("Duración: \(duration) min", value: $duration, in: 5...240, step: 5)
                if !calendars.isEmpty {
                    Picker("Calendario", selection: $calendarID) {
                        Text("Elige un calendario").tag("")
                        ForEach(calendars, id: \.calendarIdentifier) { calendar in
                            Text("\(calendar.title) · \(calendar.source.title)").tag(calendar.calendarIdentifier)
                        }
                    }
                }
                Button("Elegir calendario") { Task { await choose() } }.disabled(working)
                if !feedback.isEmpty { Text(feedback).foregroundStyle(.secondary) }
                Button("Guardar evento", action: save)
                    .disabled(working || title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || calendars.allSatisfy { $0.calendarIdentifier != calendarID })
            }
            .navigationTitle("¿Guardar este evento?")
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancelar") { dismiss() } } }
            .task { if EKEventStore.authorizationStatus(for: .event) == .fullAccess { refreshCalendars() } }
        }
    }

    private func refreshCalendars() {
        calendars = store.calendars(for: .event).filter { $0.allowsContentModifications }
        if calendars.allSatisfy({ $0.calendarIdentifier != calendarID }) { calendarID = "" }
    }

    @MainActor
    private func choose() async {
        working = true
        defer { working = false }
        do {
            guard try await store.requestFullAccessToEvents() else {
                feedback = "Permite Calendario para Duck en Ajustes del iPhone."
                return
            }
            refreshCalendars()
            feedback = calendars.isEmpty ? "Añade tu cuenta de Google a Calendario del iPhone para guardar allí." : ""
        } catch { feedback = error.localizedDescription }
    }

    private func save() {
        guard let calendar = calendars.first(where: { $0.calendarIdentifier == calendarID }) else { return }
        let event = EKEvent(eventStore: store)
        event.calendar = calendar
        event.title = title.trimmingCharacters(in: .whitespacesAndNewlines)
        event.startDate = start
        event.endDate = start.addingTimeInterval(Double(duration * 60))
        do {
            try store.save(event, span: .thisEvent, commit: true)
            onSave(calendar.title)
        } catch { feedback = error.localizedDescription }
    }
}

private func makeDuckScene() -> SCNScene {
    let scene = SCNScene()
    let duck = SCNNode()
    duck.eulerAngles.y = -0.29
    scene.rootNode.addChildNode(duck)

    func sphere(_ x: Float, _ y: Float, _ z: Float, _ sx: Float, _ sy: Float, _ sz: Float, _ color: CGColor) {
        let geometry = SCNSphere(radius: 1)
        geometry.segmentCount = 28
        let material = SCNMaterial()
        material.diffuse.contents = color
        material.specular.contents = CGColor(red: 1, green: 1, blue: 1, alpha: 1)
        geometry.materials = [material]
        let node = SCNNode(geometry: geometry)
        node.position = SCNVector3(x, y, z)
        node.scale = SCNVector3(sx, sy, sz)
        duck.addChildNode(node)
    }

    let yellow = CGColor(red: 1, green: 0.78, blue: 0.18, alpha: 1)
    let orange = CGColor(red: 0.98, green: 0.48, blue: 0.12, alpha: 1)
    let eye = CGColor(red: 0.07, green: 0.10, blue: 0.07, alpha: 1)
    sphere(0, 0, 0, 0.73, 0.58, 0.56, yellow)
    sphere(0, 0.69, 0.14, 0.55, 0.53, 0.48, yellow)
    sphere(0, 0.53, 0.61, 0.32, 0.12, 0.25, orange)
    for side: Float in [-1, 1] {
        sphere(side * 0.24, 0.81, 0.57, 0.072, 0.085, 0.042, eye)
        sphere(side * 0.64, 0.06, 0.07, 0.16, 0.30, 0.31, yellow)
        sphere(side * 0.30, -0.51, 0.23, 0.25, 0.07, 0.27, orange)
    }

    let camera = SCNNode()
    camera.camera = SCNCamera()
    camera.position = SCNVector3(0, 0.36, 3.9)
    scene.rootNode.addChildNode(camera)
    let ambient = SCNNode()
    ambient.light = SCNLight()
    ambient.light?.type = .ambient
    ambient.light?.intensity = 580
    scene.rootNode.addChildNode(ambient)
    let key = SCNNode()
    key.light = SCNLight()
    key.light?.type = .omni
    key.light?.intensity = 870
    key.position = SCNVector3(-2, 2, 3)
    scene.rootNode.addChildNode(key)
    return scene
}
