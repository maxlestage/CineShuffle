import Foundation

/// Le tirage au sort et le moteur de suggestions hors ligne : la même logique que le
/// site (src/catalog.rs), sur la sélection embarquée, pour que l'iPhone fonctionne
/// sans réseau. En ligne, l'application interroge l'API (tout le catalogue, par Wikidata).
enum Engine {
    /// Le nombre de titres d'un tirage.
    static let picksCount = 5

    /// Les valeurs sûres de la sélection embarquée, en tête des candidats.
    static let classics: [Int] = [27, 8, 44, 6, 42, 21, 61, 49, 9, 25, 62, 15, 60, 32]

    /// Le générateur splitmix64 du site.
    struct Rng {
        private var state: UInt64

        init(seed: UInt64) { state = seed }

        mutating func next() -> UInt64 {
            state = state &+ 0x9E37_79B9_7F4A_7C15
            var z = state
            z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
            z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
            return z ^ (z >> 31)
        }

        mutating func below(_ n: Int) -> Int {
            Int(next() % UInt64(n))
        }
    }

    /// La graine de la sélection du jour : le numéro du jour depuis 1970.
    static func daySeed(_ date: Date = .now) -> UInt64 {
        UInt64(max(0, date.timeIntervalSince1970)) / 86_400
    }

    /// Tire `count` titres sans doublon parmi ceux de `platform` et `kind` (`nil` : tous).
    static func picks(
        from catalog: [Title],
        seed: UInt64,
        platform: Platform? = nil,
        kind: Kind? = nil,
        count: Int = picksCount
    ) -> [Title] {
        var pool = catalog.filter { title in
            (platform.map { title.platforms.contains($0) } ?? true) && (kind == nil || title.kind == kind)
        }
        var rng = Rng(seed: seed ^ 0x5E_ED0F_F11E)
        let n = min(count, pool.count)
        for i in 0..<n {
            let j = i + rng.below(pool.count - i)
            pool.swapAt(i, j)
        }
        return Array(pool.prefix(n))
    }

    /// La similarité d'un titre et d'un titre voté (0 à 1).
    static func similarity(_ a: Title, _ b: Seen) -> Double {
        let ga = Set(a.genres)
        let gb = Set(b.genres)
        let union = ga.union(gb).count
        let jaccard = union == 0 ? 0 : Double(ga.intersection(gb).count) / Double(union)
        var score = jaccard * 0.8
        if a.kind == b.kind { score += 0.12 }
        if !Set(a.platforms).isDisjoint(with: b.platforms) { score += 0.08 }
        return score
    }

    /// La sélection embarquée, les valeurs sûres d'abord.
    static func candidates(_ catalog: [Title]) -> [Title] {
        func rank(_ t: Title) -> Int {
            let id = Int(t.key.replacingOccurrences(of: "local:", with: "")) ?? 0
            return classics.firstIndex(of: id) ?? (classics.count + id)
        }
        return catalog.sorted { rank($0) < rank($1) }
    }

    /// Les `count` meilleures suggestions pour `taste` parmi `candidates` (rangés par
    /// popularité), sans les titres déjà votés.
    static func recommend(
        from candidates: [Title],
        taste: Taste,
        count: Int = 6,
        popular: String = "Une valeur sûre du catalogue"
    ) -> [Suggestion] {
        let liked = taste.liked
        let disliked = taste.disliked
        let eligible = candidates
            .filter { taste.votes[$0.key] == nil }
            .filter { taste.platforms.isEmpty || $0.platforms.isEmpty || !taste.platforms.isDisjoint(with: $0.platforms) }
            .filter { taste.kind == nil || $0.kind == taste.kind }
        let scored = eligible.enumerated().map { rank, title -> (Int, Suggestion) in
            let best = liked.map { (similarity(title, $0), $0) }.max { $0.0 < $1.0 }
            let fromLikes = liked.reduce(0) { $0 + similarity(title, $1) }
            let fromDislikes = disliked.reduce(0) { $0 + similarity(title, $1) }
            let shared = title.genres.filter { taste.genres.contains($0) }
            let fromGenres = Double(shared.count) * 0.6
            let popularity = max(0, 0.3 - Double(rank) * 0.02)
            let score = fromLikes * 1.5 - fromDislikes + fromGenres + popularity
            let reason: String
            if let best, best.0 >= 0.35 {
                reason = "Parce que vous avez aimé « \(best.1.name) »"
            } else if !shared.isEmpty {
                reason = "Dans vos genres : " + shared.map(Genre.label).joined(separator: ", ")
            } else {
                reason = popular
            }
            return (rank, Suggestion(title: title, score: score, reason: reason))
        }
        return scored
            .sorted { $0.1.score == $1.1.score ? $0.0 < $1.0 : $0.1.score > $1.1.score }
            .prefix(count)
            .map(\.1)
    }
}

/// La sélection embarquée dans l'application (catalog.json, généré depuis le serveur).
enum Catalog {
    static func load(bundle: Bundle = .main) -> [Title] {
        guard
            let url = bundle.url(forResource: "catalog", withExtension: "json"),
            let data = try? Data(contentsOf: url),
            let titles = try? JSONDecoder().decode([Title].self, from: data)
        else { return [] }
        return titles
    }
}
