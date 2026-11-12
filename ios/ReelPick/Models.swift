import Foundation

/// Une plateforme de streaming.
enum Platform: String, Codable, CaseIterable, Identifiable, Sendable {
    case netflix
    case hboMax = "hbo-max"
    case disneyPlus = "disney-plus"

    var id: String { rawValue }

    var label: String {
        switch self {
        case .netflix: "Netflix"
        case .hboMax: "HBO Max"
        case .disneyPlus: "Disney+"
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
}

/// Un film ou une série du catalogue (le JSON de `GET /api/titles`).
struct Title: Codable, Identifiable, Hashable, Sendable {
    let id: Int
    let name: String
    let year: Int
    let kind: Kind
    let platform: Platform
    let platformLabel: String
    let genres: [String]
    let genreLabels: [String]
    let length: String
    let pitch: String
    let emoji: String
    let hue: Int

    enum CodingKeys: String, CodingKey {
        case id, name, year, kind, platform, genres, length, pitch, emoji, hue
        case platformLabel = "platform_label"
        case genreLabels = "genre_labels"
    }

    /// `2019 · Film · 2 h 13`.
    var meta: String { "\(year) · \(kind.label) · \(length)" }
}

/// Les votes de la communauté pour un titre (`GET /api/votes/stats`).
struct Stat: Codable, Hashable, Sendable {
    let title: Int
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

/// Une suggestion du moteur.
struct Suggestion: Identifiable, Hashable, Sendable {
    let title: Title
    let score: Double
    let reason: String

    var id: Int { title.id }
}

/// Les goûts de la personne, gardés sur l'iPhone.
struct Taste: Codable, Equatable, Sendable {
    /// Les votes : `true` aimé, `false` pas aimé.
    var votes: [Int: Bool] = [:]
    /// Les plateformes (vide : toutes).
    var platforms: Set<Platform> = []
    /// Les genres préférés.
    var genres: Set<String> = []
    /// Films, séries, ou les deux (`nil`).
    var kind: Kind?

    var isEmpty: Bool { self == Taste() }

    var liked: [Int] { votes.filter { $0.value }.map(\.key).sorted() }
    var disliked: [Int] { votes.filter { !$0.value }.map(\.key).sorted() }

    /// Vote pour `title` ; voter deux fois la même chose retire le vote.
    /// Renvoie le vote d'avant et le nouveau.
    @discardableResult
    mutating func toggleVote(_ title: Int, liked: Bool) -> (before: Bool?, now: Bool?) {
        let before = votes[title]
        if before == liked {
            votes[title] = nil
            return (before, nil)
        }
        votes[title] = liked
        return (before, liked)
    }
}
