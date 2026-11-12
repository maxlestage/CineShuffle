import SwiftUI

/// L'écran du tirage : les filtres, les 5 cartes et le bouton « Mélanger ».
struct PickerScreen: View {
    @Environment(AppModel.self) private var model
    @State private var diceTurns = 0.0

    var body: some View {
        @Bindable var model = model
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    header
                    Picker("Plateforme", selection: $model.platform) {
                        Text("Toutes").tag(Platform?.none)
                        ForEach(Platform.allCases) { platform in
                            Text(platform.shortLabel).tag(Optional(platform))
                        }
                    }
                    .pickerStyle(.segmented)

                    Picker("Format", selection: $model.kind) {
                        Text("Les deux").tag(Kind?.none)
                        ForEach(Kind.allCases) { kind in
                            Text(kind.plural).tag(Optional(kind))
                        }
                    }
                    .pickerStyle(.segmented)

                    LazyVStack(spacing: 16) {
                        ForEach(Array(model.picks.enumerated()), id: \.element.id) { index, title in
                            TitleCard(title: title)
                                .transition(
                                    .asymmetric(
                                        insertion: .offset(y: 60).combined(with: .opacity).combined(with: .scale(scale: 0.92)),
                                        removal: .opacity
                                    )
                                )
                                .animation(.spring(response: 0.55, dampingFraction: 0.75).delay(Double(index) * 0.07), value: model.reshuffleRound)
                        }
                    }
                    .id(model.reshuffleRound)

                    Button {
                        withAnimation(.spring(response: 0.5, dampingFraction: 0.6)) {
                            diceTurns += 1
                            model.shuffle()
                        }
                    } label: {
                        Label {
                            Text("Mélanger les 5 titres").bold()
                        } icon: {
                            Text("🎲").rotationEffect(.degrees(diceTurns * 200))
                        }
                        .frame(maxWidth: .infinity, minHeight: 50)
                    }
                    .buttonStyle(.borderedProminent)
                    .buttonBorderShape(.capsule)
                    .sensoryFeedback(.impact(weight: .medium), trigger: model.reshuffleRound)

                    Text(voteCountText)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                        .frame(maxWidth: .infinity)
                        .multilineTextAlignment(.center)
                }
                .padding()
            }
            .navigationTitle("CineShuffle")
            .refreshable { await model.start() }
            .overlay(alignment: .top) {
                if model.loading {
                    ProgressView().padding(8).background(.thinMaterial, in: Capsule()).padding(.top, 4)
                }
            }
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("NETFLIX · PRIME VIDEO · DISNEY+ · HBO MAX")
                .font(.caption.weight(.bold))
                .tracking(1.5)
                .foregroundStyle(Theme.cyan)
            Text("Ce soir, on regarde quoi ?")
                .font(.title2.bold())
            Text("Cinq titres au hasard dans tout le catalogue, avec affiche, résumé et acteurs. Votez pour ceux que vous avez vus : vos suggestions s'affinent.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
    }

    private var voteCountText: String {
        switch model.taste.votes.count {
        case 0: "Votez pour les titres que vous avez vus."
        case 1: "1 vote · vos suggestions s'affinent dans « Pour vous »."
        case let n: "\(n) votes · vos suggestions s'affinent dans « Pour vous »."
        }
    }
}

/// Une carte : l'affiche 2D, les informations et les boutons de vote.
struct TitleCard: View {
    @Environment(AppModel.self) private var model
    let title: Title

    var body: some View {
        let vote = model.vote(for: title)
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .top, spacing: 14) {
                PosterView(title: title)
                    .frame(width: 104, height: 150)
                    .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
                VStack(alignment: .leading, spacing: 6) {
                    Text(title.name).font(.headline)
                    Text(title.meta).font(.caption.weight(.semibold)).foregroundStyle(Theme.cyan)
                    if let rating = title.rating {
                        Label(String(format: "%.1f", rating), systemImage: "star.fill")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(.yellow)
                    }
                    Text(title.overview.isEmpty ? "Résumé bientôt disponible." : title.overview)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .lineLimit(6)
                    if let cast = title.castLine {
                        Text(cast).font(.caption).foregroundStyle(.primary)
                    }
                    if let url = title.linkURL {
                        Link("Résumé : Wikipédia ↗", destination: url)
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    Text(title.genreLabels)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .padding(14)

            Divider()

            VStack(alignment: .leading, spacing: 8) {
                Text("Vous l'avez vu ?").font(.caption.bold()).foregroundStyle(.secondary)
                HStack(spacing: 10) {
                    VoteButton(icon: "👍", label: "Aimé", pressed: vote == true, color: .green) {
                        model.toggleVote(title, liked: true)
                    }
                    .accessibilityLabel("Vu et aimé : \(title.name)")
                    VoteButton(icon: "👎", label: "Pas aimé", pressed: vote == false, color: .pink) {
                        model.toggleVote(title, liked: false)
                    }
                    .accessibilityLabel("Vu, pas aimé : \(title.name)")
                }
                if let summary = model.stats[title.id]?.summary {
                    Text(summary).font(.caption).foregroundStyle(.secondary)
                }
            }
            .padding(14)
        }
        .background(.background.secondary, in: RoundedRectangle(cornerRadius: 20, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 20, style: .continuous)
                .strokeBorder(vote == true ? Color.green : vote == false ? Color.pink.opacity(0.7) : Color.secondary.opacity(0.2))
        )
        .rotationEffect(.degrees(vote == true ? -0.6 : 0))
        .animation(.spring(response: 0.35, dampingFraction: 0.5), value: vote)
    }
}

/// Un bouton de vote, avec un petit rebond quand il est pressé.
struct VoteButton: View {
    let icon: String
    let label: String
    let pressed: Bool
    let color: Color
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Text(icon)
                    .scaleEffect(pressed ? 1.25 : 1)
                    .rotationEffect(.degrees(pressed ? -10 : 0))
                Text(label).bold()
            }
            .frame(maxWidth: .infinity, minHeight: 44)
            .background(pressed ? color.opacity(0.22) : Color.secondary.opacity(0.1), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(pressed ? color : .clear))
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(pressed ? .isSelected : [])
        .sensoryFeedback(.selection, trigger: pressed)
        .animation(.spring(response: 0.3, dampingFraction: 0.45), value: pressed)
    }
}

/// L'affiche 2D d'un titre : un dégradé, des anneaux qui tournent et l'emoji qui flotte.
struct PosterView: View {
    let title: Title
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var spin = false
    @State private var float = false

    var body: some View {
        ZStack(alignment: .topLeading) {
            LinearGradient(
                colors: [Theme.hue(title.hue, brightness: 0.9), Theme.hue(title.hue, shift: 50, brightness: 0.45)],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            GeometryReader { proxy in
                let size = max(proxy.size.width, proxy.size.height)
                // L'affiche de Wikipédia recouvre l'affiche dessinée une fois chargée.
                ZStack {
                    Circle()
                        .strokeBorder(.white.opacity(0.22), style: StrokeStyle(lineWidth: 2, dash: [6, 6]))
                        .frame(width: size * 1.1, height: size * 1.1)
                        .rotationEffect(.degrees(spin ? 360 : 0))
                    Circle()
                        .strokeBorder(.white.opacity(0.12), lineWidth: 2)
                        .frame(width: size * 0.6, height: size * 0.6)
                    Text(title.emoji)
                        .font(.system(size: 44))
                        .shadow(color: .black.opacity(0.4), radius: 6, y: 6)
                        .offset(y: float ? -5 : 5)
                }
                .frame(width: proxy.size.width, height: proxy.size.height)
                if let url = title.posterURL {
                    AsyncImage(url: url, transaction: Transaction(animation: .easeOut(duration: 0.4))) { phase in
                        if let image = phase.image {
                            image.resizable().scaledToFill().transition(.opacity)
                        }
                    }
                    .frame(width: proxy.size.width, height: proxy.size.height)
                    .clipped()
                }
            }
            VStack(alignment: .leading, spacing: 4) {
                ForEach(title.platforms) { platform in
                    Text(platform.label)
                        .font(.caption2.weight(.heavy))
                        .foregroundStyle(.white)
                        .padding(.horizontal, 7)
                        .padding(.vertical, 3)
                        .background(Theme.color(for: platform), in: Capsule())
                }
            }
            .padding(7)
        }
        .accessibilityHidden(true)
        .onAppear {
            guard !reduceMotion else { return }
            withAnimation(.linear(duration: 18).repeatForever(autoreverses: false)) { spin = true }
            withAnimation(.easeInOut(duration: 1.8).repeatForever(autoreverses: true)) { float = true }
        }
    }
}
