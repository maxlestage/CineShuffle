import SwiftUI

@main
struct ReelPickApp: App {
    @State private var model = AppModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(model)
                .tint(Theme.accent)
                .task { await model.start() }
        }
    }
}

/// Les couleurs du site.
enum Theme {
    static let accent = Color(red: 1.0, green: 0.30, blue: 0.43)
    static let violet = Color(red: 0.66, green: 0.33, blue: 0.97)
    static let cyan = Color(red: 0.13, green: 0.83, blue: 0.93)
    static let gradient = LinearGradient(colors: [accent, violet], startPoint: .topLeading, endPoint: .bottomTrailing)

    static func color(for platform: Platform) -> Color {
        switch platform {
        case .netflix: Color(red: 0.90, green: 0.04, blue: 0.08)
        case .hboMax: Color(red: 0.49, green: 0.23, blue: 0.93)
        case .primeVideo: Color(red: 0.0, green: 0.66, blue: 0.88)
        case .disneyPlus: Color(red: 0.05, green: 0.49, blue: 0.95)
        }
    }

    static func hue(_ value: Int, shift: Int = 0, saturation: Double = 0.8, brightness: Double = 0.8) -> Color {
        Color(hue: Double((value + shift) % 360) / 360, saturation: saturation, brightness: brightness)
    }
}

struct ContentView: View {
    var body: some View {
        TabView {
            Tab("Sélection", systemImage: "dice") {
                PickerScreen()
            }
            Tab("Pour vous", systemImage: "sparkles") {
                SuggestionsScreen()
            }
            Tab("Réglages", systemImage: "gearshape") {
                SettingsScreen()
            }
        }
    }
}
