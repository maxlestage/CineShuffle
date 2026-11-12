import Foundation

/// Une plateforme de streaming.
enum Platform: String, Codable, CaseIterable, Identifiable, Sendable {
    case netflix
    case primeVideo = "prime-video"
    case disneyPlus = "disney-plus"
    case hboMax = "hbo-max"

    var id: String { rawValue }

    /// Le nom court (filtres segmentés).
    var shortLabel: String {
        switch self {
        case .netflix: "Netflix"
        case .primeVideo: "Prime"
        case .disneyPlus: "Disney+"
        case .hboMax: "HBO"
        }
    }

    var label: String {
        switch self {
        case .netflix: "Netflix"
        case .primeVideo: "Prime Video"
        case .disneyPlus: "Disney+"
        case .hboMax: "HBO Max"
        }
    }
}

/// Film ou série.
enum Kind: String, Codable, CaseIterable, Identifiable, Sendable {
    case film
    case serie

    var id: String { rawValue }

    var label: String {
        switch self {
        case .film: "Film"
        case .serie: "Série"
        }
    }

    var plural: String {
        switch self {
        case .film: "Films"
        case .serie: "Séries"
        }
    }
}

/// Un genre (identifiant et nom affiché), dans l'ordre du site.
struct Genre: Identifiable, Hashable, Sendable {
    let id: String
    let label: String

    static let all: [Genre] = [
        Genre(id: "action", label: "Action"),
        Genre(id: "aventure", label: "Aventure"),
        Genre(id: "animation", label: "Animation"),
        Genre(id: "comedie", label: "Comédie"),
        Genre(id: "policier", label: "Policier"),
        Genre(id: "documentaire", label: "Documentaire"),
        Genre(id: "drame", label: "Drame"),
        Genre(id: "famille", label: "Famille"),
        Genre(id: "fantastique", label: "Fantastique"),
        Genre(id: "historique", label: "Historique"),
        Genre(id: "horreur", label: "Horreur"),
        Genre(id: "mystere", label: "Mystère"),
        Genre(id: "romance", label: "Romance"),
        Genre(id: "sf", label: "Science-fiction"),
        Genre(id: "thriller", label: "Thriller"),
    ]

    static func label(_ id: String) -> String {
        all.first { $0.id == id }?.label ?? id
    }
}

/// Un film ou une série (le JSON de l'API CineShuffle : Wikidata ou sélection embarquée).
struct Title: Codable, Identifiable, Hashable, Sendable {
    /// `wd:Q83495` (Wikidata) ou `local:12` (sélection embarquée).
    let key: String
    let name: String
    var year: Int?
    let kind: Kind
    var platforms: [Platform] = []
    var genres: [String] = []
    var overview: String = ""
    /// L'adresse de l'affiche (Wikipédia).
    var poster: String?
    /// Les acteurs principaux.
    var cast: [String] = []
    var length: String?
    var rating: Double?
    /// L'article Wikipédia du résumé (crédit CC BY-SA).
    var link: String?
    let emoji: String
    let hue: Int

    var id: String { key }

    enum CodingKeys: String, CodingKey {
        case key, name, year, kind, platforms, genres, overview, poster, cast, length, rating, link, emoji, hue
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        key = try c.decode(String.self, forKey: .key)
        name = try c.decode(String.self, forKey: .name)
        year = try c.decodeIfPresent(Int.self, forKey: .year)
        kind = try c.decode(Kind.self, forKey: .kind)
        // Une plateforme inconnue (ajoutée plus tard au serveur) est ignorée.
        let slugs = try c.decodeIfPresent([String].self, forKey: .platforms) ?? []
        platforms = slugs.compactMap(Platform.init(rawValue:))
        genres = try c.decodeIfPresent([String].self, forKey: .genres) ?? []
        overview = try c.decodeIfPresent(String.self, forKey: .overview) ?? ""
        poster = try c.decodeIfPresent(String.self, forKey: .poster)
        cast = try c.decodeIfPresent([String].self, forKey: .cast) ?? []
        length = try c.decodeIfPresent(String.self, forKey: .length)
        rating = try c.decodeIfPresent(Double.self, forKey: .rating)
        link = try c.decodeIfPresent(String.self, forKey: .link)
        emoji = try c.decodeIfPresent(String.self, forKey: .emoji) ?? "🎬"
        hue = try c.decodeIfPresent(Int.self, forKey: .hue) ?? 0
    }

    /// `2019 · Film · 2 h 13`.
    var meta: String {
        var parts: [String] = []
        if let year { parts.append(String(year)) }
        parts.append(kind.label)
        if let length { parts.append(length) }
        return parts.joined(separator: " · ")
    }

    /// `Avec A, B et C`.
    var castLine: String? {
        guard let last = cast.last else { return nil }
        if cast.count == 1 { return "Avec \(last)" }
        return "Avec \(cast.dropLast().joined(separator: ", ")) et \(last)"
    }

    var posterURL: URL? { poster.flatMap(URL.init(string:)) }

    var linkURL: URL? { link.flatMap(URL.init(string:)) }

    var platformLabels: String { platforms.map(\.label).joined(separator: ", ") }

    var genreLabels: String { genres.map(Genre.label).joined(separator: " · ") }

    /// Ce que le moteur garde d'un titre voté.
    var seen: Seen { Seen(key: key, name: name, kind: kind, genres: genres, platforms: platforms) }
}

/// Un titre voté, tel que le moteur de suggestions le compare aux autres.
struct Seen: Codable, Hashable, Sendable {
    let key: String
    let name: String
    let kind: Kind
    var genres: [String]
    var platforms: [Platform]
}

/// Un vote.
struct VoteEntry: Codable, Hashable, Sendable {
    let seen: Seen
    let liked: Bool
}

/// Les votes de la communauté pour un titre (`GET /api/votes/stats`).
struct Stat: Codable, Hashable, Sendable {
    let title: String
    let likes: Int
    let dislikes: Int

    var total: Int { likes + dislikes }

    /// `87 % aiment · 12 votes`.
    var summary: String? {
        guard total > 0 else { return nil }
        let percent = likes * 100 / total
        return "\(percent) % aiment · \(total) vote\(total > 1 ? "s" : "")"
    }
}

/// Une suggestion (`POST /api/recommendations`, ou calculée sur l'iPhone).
struct Suggestion: Codable, Identifiable, Hashable, Sendable {
    let title: Title
    let score: Double
    let reason: String

    var id: String { title.key }
}

/// Les goûts de la personne, gardés sur l'iPhone.
struct Taste: Codable, Equatable, Sendable {
    /// Les votes, par clé de titre.
    var votes: [String: VoteEntry] = [:]
    /// Les plateformes (vide : toutes).
    var platforms: Set<Platform> = []
    /// Les genres préférés.
    var genres: Set<String> = []
    /// Films, séries, ou les deux (`nil`).
    var kind: Kind?

    var isEmpty: Bool { self == Taste() }

    var liked: [Seen] { votes.values.filter(\.liked).map(\.seen).sorted { $0.key < $1.key } }
    var disliked: [Seen] { votes.values.filter { !$0.liked }.map(\.seen).sorted { $0.key < $1.key } }

    func vote(_ key: String) -> Bool? { votes[key]?.liked }

    /// Vote pour `title` ; voter deux fois la même chose retire le vote.
    /// Renvoie le vote d'avant et le nouveau.
    @discardableResult
    mutating func toggleVote(_ title: Title, liked: Bool) -> (before: Bool?, now: Bool?) {
        let before = votes[title.key]?.liked
        if before == liked {
            votes[title.key] = nil
            return (before, nil)
        }
        votes[title.key] = VoteEntry(seen: title.seen, liked: liked)
        return (before, liked)
    }

    /// Le profil envoyé à `POST /api/recommendations`.
    func profile(limit: Int) -> Profile {
        Profile(
            liked: liked,
            disliked: disliked,
            platforms: Platform.allCases.filter(platforms.contains),
            genres: Genre.all.map(\.id).filter(genres.contains),
            kind: kind,
            limit: limit
        )
    }
}

/// Le corps de `POST /api/recommendations`.
struct Profile: Encodable, Sendable {
    let liked: [Seen]
    let disliked: [Seen]
    let platforms: [Platform]
    let genres: [String]
    let kind: Kind?
    let limit: Int
}
