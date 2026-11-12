import SwiftUI

/// Les réglages : le serveur, l'état de la connexion et les liens.
struct SettingsScreen: View {
    @Environment(AppModel.self) private var model
    @State private var draft = ""

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("https://…", text: $draft)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .onSubmit { model.serverURL = draft }
                    Button("Enregistrer") { model.serverURL = draft }
                        .disabled(draft == model.serverURL)
                    LabeledContent("État") {
                        Label(
                            model.online ? "Connecté" : "Hors ligne",
                            systemImage: model.online ? "checkmark.circle.fill" : "wifi.slash"
                        )
                        .foregroundStyle(model.online ? .green : .secondary)
                    }
                } header: {
                    Text("Serveur ReelPick")
                } footer: {
                    Text("L'adresse de l'application Heroku. Sans serveur, le tirage et les suggestions fonctionnent quand même : seuls les votes de la communauté manquent.")
                }

                Section("Catalogue") {
                    LabeledContent("Source", value: model.online ? "Tout le catalogue (TMDB)" : "Sélection embarquée")
                    LabeledContent("Titres hors ligne", value: "\(model.catalog.count)")
                    LabeledContent("Mes votes", value: "\(model.taste.votes.count)")
                }

                Section("À propos") {
                    Link("Code source", destination: URL(string: "https://github.com/maxlestage/ReelPick")!)
                    Link("Confidentialité", destination: URL(string: "https://maxlestage.github.io/ReelPick/confidentialite/")!)
                    Text("Ce produit utilise l'API TMDB mais n'est ni approuvé ni certifié par TMDB. Disponibilités : JustWatch.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                    Text("Netflix, Prime Video, Disney+ et HBO Max sont des marques de leurs propriétaires respectifs ; ReelPick n'est affilié à aucune d'elles.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            }
            .navigationTitle("Réglages")
            .onAppear { draft = model.serverURL }
        }
    }
}
