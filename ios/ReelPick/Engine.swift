import Foundation

/// Le tirage au sort et le moteur de suggestions : la même logique que le site
/// (src/catalog.rs), pour que l'iPhone donne la même sélection du jour, même hors ligne.
enum Engine {
    /// Le nombre de titres d'un tirage.
    static let picksCount = 5

    /// Les valeurs sûres, montrées tant qu'il n'y a ni vote ni préférence.
    static let classics: [Int] = [27, 8, 44, 6, 42, 21, 49, 9, 25, 15, 60, 32]

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
            (platform == nil || title.platform == platform) && (kind == nil || title.kind == kind)
        }
        var rng = Rng(seed: seed ^ 0x5E_ED0F_F11E)
        let n = min(count, pool.count)
        for i in 0..<n {
            let j = i + rng.below(pool.count - i)
            pool.swapAt(i, j)
        }
        return Array(pool.prefix(n))
    }

    /// La similarité de deux titres (0 à 1).
    static func similarity(_ a: Title, _ b: Title) -> Double {
        let ga = Set(a.genres)
        let gb = Set(b.genres)
        let union = ga.union(gb).count
        let jaccard = union == 0 ? 0 : Double(ga.intersection(gb).count) / Double(union)
        var score = jaccard * 0.8
        if a.kind == b.kind { score += 0.12 }
        if a.platform == b.platform { score += 0.08 }
        return score
    }

    /// Les `count` meilleures suggestions pour `taste`, parmi les titres pas encore votés.
    static func recommend(from catalog: [Title], taste: Taste, count: Int = 6) -> [Suggestion] {
        let byId = Dictionary(uniqueKeysWithValues: catalog.map { ($0.id, $0) })
        let liked = taste.liked.compactMap { byId[$0] }
        let disliked = taste.disliked.compactMap { byId[$0] }
        let labels = Dictionary(uniqueKeysWithValues: Genre.all.map { ($0.id, $0.label) })

        let found = catalog
            .filter { taste.votes[$0.id] == nil }
            .filter { taste.platforms.isEmpty || taste.platforms.contains($0.platform) }
            .filter { taste.kind == nil || $0.kind == taste.kind }
            .map { title -> Suggestion in
                let best = liked
                    .map { (similarity(title, $0), $0) }
                    .max { $0.0 < $1.0 }
                let fromLikes = liked.reduce(0) { $0 + similarity(title, $1) }
                let fromDislikes = disliked.reduce(0) { $0 + similarity(title, $1) }
                let shared = title.genres.filter { taste.genres.contains($0) }
                let fromGenres = Double(shared.count) * 0.6
                let classic = classics.firstIndex(of: title.id).map { 0.3 - Double($0) * 0.02 } ?? 0
                let score = fromLikes * 1.5 - fromDislikes + fromGenres + classic
                let reason: String
                if let best, best.0 >= 0.35 {
                    reason = "Parce que vous avez aimé « \(best.1.name) »"
                } else if !shared.isEmpty {
                    reason = "Dans vos genres : " + shared.compactMap { labels[$0] }.joined(separator: ", ")
                } else {
                    reason = "Une valeur sûre du catalogue"
                }
                return Suggestion(title: title, score: score, reason: reason)
            }
            .sorted { lhs, rhs in
                lhs.score == rhs.score ? lhs.title.id < rhs.title.id : lhs.score > rhs.score
            }
        return Array(found.prefix(count))
    }
}

/// Le catalogue embarqué dans l'application (catalog.json, généré depuis le serveur).
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
