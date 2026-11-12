import SwiftUI

/// « Pour vous » : les préférences, les suggestions et le formulaire pour proposer un titre.
struct SuggestionsScreen: View {
    @Environment(AppModel.self) private var model
    @State private var proposal = ""
    @State private var proposalPlatform: Platform = .netflix
    @State private var status = ""
    @State private var sending = false

    private let genreColumns = [GridItem(.adaptive(minimum: 104), spacing: 8)]

    var body: some View {
        @Bindable var model = model
        NavigationStack {
            Form {
                Section {
                    if model.suggestions.isEmpty {
                        Text("Plus rien à suggérer avec ces filtres : élargissez vos plateformes ou vos genres.")
                            .foregroundStyle(.secondary)
                    }
                    ForEach(Array(model.suggestions.enumerated()), id: \.element.id) { index, suggestion in
                        SuggestionRow(suggestion: suggestion)
                            .transition(.move(edge: .trailing).combined(with: .opacity))
                            .animation(.spring(response: 0.45, dampingFraction: 0.8).delay(Double(index) * 0.05), value: model.taste)
                    }
                } header: {
                    Text("Suggestions pour vous")
                } footer: {
                    Text("Calculées sur l'iPhone à partir de vos votes : genres en commun, format et plateforme.")
                }

                Section("Mes plateformes") {
                    ForEach(Platform.allCases) { platform in
                        Toggle(platform.label, isOn: Binding(
                            get: { model.taste.platforms.contains(platform) },
                            set: { _ in model.togglePlatform(platform) }
                        ))
                        .tint(Theme.color(for: platform))
                    }
                }

                Section("Je préfère") {
                    Picker("Format", selection: $model.taste.kind) {
                        Text("Les deux").tag(Kind?.none)
                        ForEach(Kind.allCases) { kind in
                            Text(kind.plural).tag(Optional(kind))
                        }
                    }
                    .pickerStyle(.segmented)
                }

                Section("Mes genres") {
                    LazyVGrid(columns: genreColumns, alignment: .leading, spacing: 8) {
                        ForEach(Genre.all) { genre in
                            let on = model.taste.genres.contains(genre.id)
                            Button {
                                withAnimation(.snappy) { model.toggleGenre(genre) }
                            } label: {
                                Text(genre.label)
                                    .font(.subheadline.weight(.semibold))
                                    .lineLimit(1)
                                    .minimumScaleFactor(0.8)
                                    .frame(maxWidth: .infinity, minHeight: 36)
                                    .foregroundStyle(on ? .white : .primary)
                                    .background(on ? AnyShapeStyle(Theme.gradient) : AnyShapeStyle(Color.secondary.opacity(0.12)), in: Capsule())
                            }
                            .buttonStyle(.plain)
                            .accessibilityAddTraits(on ? .isSelected : [])
                        }
                    }
                    .padding(.vertical, 4)
                }

                Section("Proposer un titre") {
                    TextField("Un film ou une série à ajouter", text: $proposal)
                        .textInputAutocapitalization(.words)
                        .submitLabel(.send)
                        .onSubmit(send)
                    Picker("Plateforme", selection: $proposalPlatform) {
                        ForEach(Platform.allCases) { Text($0.label).tag($0) }
                    }
                    Button(action: send) {
                        if sending { ProgressView() } else { Text("Envoyer") }
                    }
                    .disabled(sending || proposal.trimmingCharacters(in: .whitespaces).isEmpty)
                    if !status.isEmpty {
                        Text(status).font(.footnote).foregroundStyle(.green)
                    }
                }

                Section {
                    Button("Réinitialiser mes goûts", role: .destructive) {
                        withAnimation { model.resetTaste() }
                    }
                    .disabled(model.taste.isEmpty)
                }
            }
            .navigationTitle("Pour vous")
        }
    }

    private func send() {
        let text = proposal
        sending = true
        Task {
            status = await model.propose(text, platform: proposalPlatform)
            if status.hasPrefix("Merci") { proposal = "" }
            sending = false
        }
    }
}

/// Une suggestion : l'emoji, le titre, la plateforme et la raison.
struct SuggestionRow: View {
    let suggestion: Suggestion

    var body: some View {
        let title = suggestion.title
        HStack(spacing: 12) {
            Text(title.emoji)
                .font(.title)
                .frame(width: 50, height: 50)
                .background(Theme.hue(title.hue, saturation: 0.6, brightness: 0.55).opacity(0.5), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            VStack(alignment: .leading, spacing: 2) {
                Text(title.name).font(.headline)
                Text("\(title.platform.label) · \(title.kind.label) · \(title.year)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(suggestion.reason)
                    .font(.caption)
                    .foregroundStyle(Theme.cyan)
            }
        }
        .accessibilityElement(children: .combine)
    }
}
