import Foundation
import Observation

/// L'état de l'application : le tirage, les goûts, les suggestions et les votes de la
/// communauté. En ligne, les titres viennent de l'API (tout le catalogue, par Wikidata) ;
/// hors ligne, de la sélection embarquée. Les goûts sont gardés dans `UserDefaults`.
@MainActor
@Observable
final class AppModel {
    /// La sélection embarquée (secours hors ligne).
    let catalog: [Title]

    /// Les 5 titres affichés.
    private(set) var picks: [Title] = []
    /// Les suggestions affichées.
    private(set) var suggestions: [Suggestion] = []
    /// Un tirage est en cours.
    private(set) var loading = false
    /// Change à chaque nouveau tirage (pour rejouer les animations).
    private(set) var reshuffleRound = 0

    /// Le filtre de plateforme du tirage (`nil` : toutes).
    var platform: Platform? {
        didSet { if platform != oldValue { Task { await draw(seed: Self.randomSeed()) } } }
    }
    /// Le filtre de format du tirage (`nil` : films et séries).
    var kind: Kind? {
        didSet { if kind != oldValue { Task { await draw(seed: Self.randomSeed()) } } }
    }

    /// Les goûts de la personne.
    var taste: Taste {
        didSet {
            save()
            if taste != oldValue { Task { await refreshSuggestions() } }
        }
    }

    /// Les votes de la communauté, par titre.
    private(set) var stats: [String: Stat] = [:]
    /// Le serveur répond-il ?
    private(set) var online = false

    /// L'adresse du serveur.
    var serverURL: String {
        didSet {
            defaults.set(serverURL, forKey: Keys.server)
            Task { await start() }
        }
    }

    private let defaults: UserDefaults

    private enum Keys {
        static let taste = "cineshuffle.taste.v2"
        static let server = "cineshuffle.server"
    }

    init(catalog: [Title] = Catalog.load(), defaults: UserDefaults = .standard) {
        self.catalog = catalog
        self.defaults = defaults
        if let data = defaults.data(forKey: Keys.taste),
           let saved = try? JSONDecoder().decode(Taste.self, from: data) {
            taste = saved
        } else {
            taste = Taste()
        }
        serverURL = defaults.string(forKey: Keys.server) ?? AppModel.defaultServer
        picks = Engine.picks(from: catalog, seed: Engine.daySeed())
        suggestions = Engine.recommend(from: Engine.candidates(catalog), taste: taste)
    }

    /// L'adresse donnée à la compilation (Info.plist, `CineShuffleAPIBaseURL`).
    static var defaultServer: String {
        (Bundle.main.object(forInfoDictionaryKey: "CineShuffleAPIBaseURL") as? String)?
            .trimmingCharacters(in: .whitespaces) ?? ""
    }

    var api: APIClient? {
        guard let url = URL(string: serverURL), url.scheme?.hasPrefix("http") == true else { return nil }
        return APIClient(baseURL: url)
    }

    static func randomSeed() -> UInt64 { UInt64.random(in: 1...UInt64.max) }

    /// Au lancement : la sélection du jour, les suggestions et les votes du serveur.
    func start() async {
        await draw(seed: nil)
        await refreshSuggestions()
        await refreshStats()
    }

    // MARK: - Tirage

    /// Un nouveau tirage.
    func shuffle() {
        Task { await draw(seed: Self.randomSeed()) }
    }

    /// Tire 5 titres : au serveur (tout le catalogue), sinon dans la sélection embarquée.
    /// `seed == nil` : la sélection du jour.
    func draw(seed: UInt64?) async {
        loading = true
        defer { loading = false }
        var found: [Title] = []
        if let api {
            do {
                found = try await api.picks(seed: seed, platform: platform, kind: kind)
                online = true
            } catch {
                online = false
            }
        }
        if found.isEmpty {
            found = Engine.picks(from: catalog, seed: seed ?? Engine.daySeed(), platform: platform, kind: kind)
        }
        picks = found
        reshuffleRound += 1
    }

    // MARK: - Votes

    func vote(for title: Title) -> Bool? { taste.vote(title.key) }

    /// Vote (ou retire le vote) et l'envoie au serveur.
    func toggleVote(_ title: Title, liked: Bool) {
        let change = taste.toggleVote(title, liked: liked)
        guard let api else { return }
        applyLocally(title.key, before: change.before, now: change.now)
        Task {
            do {
                try await api.vote(title: title.key, previous: change.before, liked: change.now)
                online = true
            } catch {
                online = false
            }
        }
    }

    private func applyLocally(_ key: String, before: Bool?, now: Bool?) {
        let old = stats[key] ?? Stat(title: key, likes: 0, dislikes: 0)
        var likes = old.likes
        var dislikes = old.dislikes
        if before == true { likes = max(0, likes - 1) }
        if before == false { dislikes = max(0, dislikes - 1) }
        if now == true { likes += 1 }
        if now == false { dislikes += 1 }
        stats[key] = Stat(title: key, likes: likes, dislikes: dislikes)
    }

    /// Recharge les votes de la communauté.
    func refreshStats() async {
        guard let api else { return }
        do {
            let all = try await api.stats()
            stats = Dictionary(all.map { ($0.title, $0) }, uniquingKeysWith: { _, last in last })
            online = true
        } catch {
            online = false
        }
    }

    // MARK: - Suggestions

    /// Recalcule les suggestions : au serveur, sinon sur l'iPhone.
    func refreshSuggestions() async {
        let wanted = taste
        if let api {
            do {
                let found = try await api.recommendations(wanted.profile(limit: 6))
                if wanted == taste { suggestions = found }
                online = true
                return
            } catch {
                online = false
            }
        }
        suggestions = Engine.recommend(from: Engine.candidates(catalog), taste: wanted)
    }

    func togglePlatform(_ platform: Platform) {
        if taste.platforms.contains(platform) {
            taste.platforms.remove(platform)
        } else {
            taste.platforms.insert(platform)
        }
    }

    func toggleGenre(_ genre: Genre) {
        if taste.genres.contains(genre.id) {
            taste.genres.remove(genre.id)
        } else {
            taste.genres.insert(genre.id)
        }
    }

    func resetTaste() {
        taste = Taste()
    }

    /// Propose un titre ; renvoie le message à afficher.
    func propose(_ title: String, platform: Platform) async -> String {
        let clean = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard clean.count >= 2 else { return "Indiquez un titre (2 caractères au moins)." }
        guard let api else { return "Merci ! « \(clean) » est noté sur cet iPhone." }
        do {
            try await api.propose(title: clean, platform: platform)
            return "Merci ! « \(clean) » est envoyé à l'équipe."
        } catch {
            return "Le serveur ne répond pas : réessayez plus tard."
        }
    }

    private func save() {
        if let data = try? JSONEncoder().encode(taste) {
            defaults.set(data, forKey: Keys.taste)
        }
    }
}
