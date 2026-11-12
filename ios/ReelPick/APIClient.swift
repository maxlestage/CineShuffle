import Foundation

/// Le client de l'API du serveur ReelPick (le même que celui du site).
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

    /// `GET /api/votes/stats`
    func stats() async throws -> [Stat] {
        try await get("api/votes/stats")
    }

    /// `GET /api/titles`
    func titles() async throws -> [Title] {
        try await get("api/titles")
    }

    /// `POST /api/votes` : le vote (`liked`, `nil` pour le retirer) et celui qu'il remplace.
    func vote(title: Int, previous: Bool?, liked: Bool?) async throws {
        struct Body: Encodable {
            let title: Int
            let previous: Bool?
            let liked: Bool?
        }
        try await post("api/votes", body: Body(title: title, previous: previous, liked: liked))
    }

    /// `POST /api/proposals` : propose un titre à ajouter au catalogue.
    func propose(title: String, platform: Platform) async throws {
        struct Body: Encodable {
            let title: String
            let platform: String
        }
        try await post("api/proposals", body: Body(title: title, platform: platform.rawValue))
    }

    private func get<T: Decodable>(_ path: String) async throws -> T {
        var request = URLRequest(url: baseURL.appending(path: path))
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.timeoutInterval = 10
        let (data, response) = try await session.data(for: request)
        try check(response)
        return try JSONDecoder().decode(T.self, from: data)
    }

    private func post(_ path: String, body: some Encodable) async throws {
        var request = URLRequest(url: baseURL.appending(path: path))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.timeoutInterval = 10
        request.httpBody = try JSONEncoder().encode(body)
        let (_, response) = try await session.data(for: request)
        try check(response)
    }

    private func check(_ response: URLResponse) throws {
        guard let http = response as? HTTPURLResponse else { return }
        guard (200..<300).contains(http.statusCode) else { throw APIError.status(http.statusCode) }
    }
}
