import Foundation
import Observation

/// L'état de l'application : le catalogue, le tirage, les goûts et les votes de la
/// communauté. Les goûts sont gardés dans `UserDefaults`.
@MainActor
@Observable
final class AppModel {
    /// Le catalogue embarqué.
    let catalog: [Title]

    /// La graine du tirage (la sélection du jour au lancement).
    var seed: UInt64 = Engine.daySeed()
    /// Le filtre de plateforme du tirage (`nil` : toutes).
    var platform: Platform? {
        didSet { reshuffleRound += 1 }
    }
    /// Le filtre de format du tirage (`nil` : films et séries).
    var kind: Kind? {
        didSet { reshuffleRound += 1 }
    }
    /// Change à chaque nouveau tirage (pour rejouer les animations).
    private(set) var reshuffleRound = 0

    /// Les goûts de la personne.
    var taste: Taste {
        didSet { save() }
    }

    /// Les votes de la communauté, par titre.
    private(set) var stats: [Int: Stat] = [:]
    /// Le serveur répond-il ?
    private(set) var online = false

    /// L'adresse du serveur.
    var serverURL: String {
        didSet {
            defaults.set(serverURL, forKey: Keys.server)
            Task { await refreshStats() }
        }
    }

    private let defaults: UserDefaults

    private enum Keys {
        static let taste = "reelpick.taste"
        static let server = "reelpick.server"
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
    }

    /// L'adresse donnée à la compilation (Info.plist, `ReelPickAPIBaseURL`).
    static var defaultServer: String {
        (Bundle.main.object(forInfoDictionaryKey: "ReelPickAPIBaseURL") as? String)?
            .trimmingCharacters(in: .whitespaces) ?? ""
    }

    var api: APIClient? {
        guard let url = URL(string: serverURL), url.scheme?.hasPrefix("http") == true else { return nil }
        return APIClient(baseURL: url)
    }

    // MARK: - Tirage

    /// Les 5 titres tirés.
    var picks: [Title] {
        Engine.picks(from: catalog, seed: seed, platform: platform, kind: kind)
    }

    /// Un nouveau tirage.
    func shuffle() {
        var next = UInt64.random(in: 1...UInt64.max)
        if next == seed { next &+= 1 }
        seed = next
        reshuffleRound += 1
    }

    // MARK: - Votes

    func vote(for title: Title) -> Bool? { taste.votes[title.id] }

    /// Vote (ou retire le vote) et l'envoie au serveur.
    func toggleVote(_ title: Title, liked: Bool) {
        let change = taste.toggleVote(title.id, liked: liked)
        applyLocally(title.id, before: change.before, now: change.now)
        guard let api else { return }
        Task {
            do {
                try await api.vote(title: title.id, previous: change.before, liked: change.now)
                online = true
            } catch {
                online = false
            }
        }
    }

    private func applyLocally(_ id: Int, before: Bool?, now: Bool?) {
        let old = stats[id] ?? Stat(title: id, likes: 0, dislikes: 0)
        var likes = old.likes
        var dislikes = old.dislikes
        if before == true { likes = max(0, likes - 1) }
        if before == false { dislikes = max(0, dislikes - 1) }
        if now == true { likes += 1 }
        if now == false { dislikes += 1 }
        stats[id] = Stat(title: id, likes: likes, dislikes: dislikes)
    }

    /// Recharge les votes de la communauté.
    func refreshStats() async {
        guard let api else {
            online = false
            return
        }
        do {
            let all = try await api.stats()
            stats = Dictionary(uniqueKeysWithValues: all.map { ($0.title, $0) })
            online = true
        } catch {
            online = false
        }
    }

    // MARK: - Suggestions

    var suggestions: [Suggestion] {
        Engine.recommend(from: catalog, taste: taste)
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
