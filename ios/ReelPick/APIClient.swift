import Foundation

/// Le client de l'API du serveur ReelPick (la même que celle du site).
struct APIClient: Sendable {
    let baseURL: URL
    var session: URLSession = .shared

    enum APIError: Error, LocalizedError {
        case status(Int)

        var errorDescription: String? {
            switch self {
            case .status(let code): "Le serveur a répondu \(code)."
            }
        }
    }

    /// `GET /api/picks` : 5 titres au hasard dans tout le catalogue (TMDB).
    func picks(seed: UInt64?, platform: Platform?, kind: Kind?) async throws -> [Title] {
        var query: [URLQueryItem] = []
        if let seed { query.append(URLQueryItem(name: "seed", value: String(seed))) }
        if let platform { query.append(URLQueryItem(name: "platform", value: platform.rawValue)) }
        if let kind { query.append(URLQueryItem(name: "kind", value: kind.rawValue)) }
        return try await get("api/picks", query: query)
    }

    /// `POST /api/recommendations`
    func recommendations(_ profile: Profile) async throws -> [Suggestion] {
        try await post("api/recommendations", body: profile)
    }

    /// `GET /api/votes/stats`
    func stats() async throws -> [Stat] {
        try await get("api/votes/stats")
    }

    /// `POST /api/votes` : le vote (`liked`, `nil` pour le retirer) et celui qu'il remplace.
    func vote(title: String, previous: Bool?, liked: Bool?) async throws {
        struct Body: Encodable {
            let title: String
            let previous: Bool?
            let liked: Bool?
        }
        let _: Stat = try await post("api/votes", body: Body(title: title, previous: previous, liked: liked))
    }

    /// `POST /api/proposals` : propose un titre.
    func propose(title: String, platform: Platform) async throws {
        struct Body: Encodable {
            let title: String
            let platform: String
        }
        struct Created: Decodable {}
        let _: Created = try await post("api/proposals", body: Body(title: title, platform: platform.rawValue))
    }

    private func get<T: Decodable>(_ path: String, query: [URLQueryItem] = []) async throws -> T {
        var url = baseURL.appending(path: path)
        if !query.isEmpty { url.append(queryItems: query) }
        var request = URLRequest(url: url)
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.timeoutInterval = 15
        let (data, response) = try await session.data(for: request)
        try check(response)
        return try JSONDecoder().decode(T.self, from: data)
    }

    private func post<T: Decodable>(_ path: String, body: some Encodable) async throws -> T {
        var request = URLRequest(url: baseURL.appending(path: path))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.timeoutInterval = 15
        request.httpBody = try JSONEncoder().encode(body)
        let (data, response) = try await session.data(for: request)
        try check(response)
        return try JSONDecoder().decode(T.self, from: data)
    }

    private func check(_ response: URLResponse) throws {
        guard let http = response as? HTTPURLResponse else { return }
        guard (200..<300).contains(http.statusCode) else { throw APIError.status(http.statusCode) }
    }
}
