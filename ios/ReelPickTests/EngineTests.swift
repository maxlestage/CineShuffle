import Foundation
import Testing
@testable import ReelPick

@MainActor
struct EngineTests {
    let catalog = Catalog.load(bundle: Bundle(for: AppModel.self))

    @Test func catalogIsBundled() {
        #expect(catalog.count == 80)
        #expect(Set(catalog.flatMap(\.platforms)) == Set(Platform.allCases))
    }

    @Test func picksAreReproducibleAndUnique() {
        let a = Engine.picks(from: catalog, seed: 42)
        #expect(a.count == 5)
        #expect(a == Engine.picks(from: catalog, seed: 42))
        #expect(Set(a.map(\.key)).count == 5)
        let prime = Engine.picks(from: catalog, seed: 7, platform: .primeVideo, kind: .film)
        #expect(prime.allSatisfy { $0.platforms == [.primeVideo] && $0.kind == .film })
    }

    /// La même graine donne la même sélection que le site (src/catalog.rs).
    @Test func picksMatchTheWebsite() {
        var rng = Engine.Rng(seed: 0)
        #expect(rng.next() == 0xE220_A839_7B1D_CDAF)
        let keys = Engine.picks(from: catalog, seed: 42).map(\.key)
        #expect(keys == ["local:68", "local:39", "local:4", "local:10", "local:25"])
    }

    @Test func suggestionsFollowTheVotes() throws {
        let candidates = Engine.candidates(catalog)
        let none = Engine.recommend(from: candidates, taste: Taste(), count: 4)
        #expect(none.first?.title.key == "local:27")

        var taste = Taste()
        let wire = try #require(catalog.first { $0.key == "local:25" })
        let sopranos = try #require(catalog.first { $0.key == "local:26" })
        taste.toggleVote(wire, liked: true)
        taste.toggleVote(sopranos, liked: true)
        let found = Engine.recommend(from: candidates, taste: taste, count: 3)
        #expect(found.allSatisfy { $0.title.genres.contains("policier") })
        #expect(found.first?.reason.hasPrefix("Parce que vous avez aimé") == true)
    }

    @Test func votesToggle() throws {
        let title = try #require(catalog.first)
        var taste = Taste()
        #expect(taste.toggleVote(title, liked: true).now == true)
        #expect(taste.toggleVote(title, liked: false).now == false)
        #expect(taste.toggleVote(title, liked: false).now == nil)
        #expect(taste.votes.isEmpty)
    }

    @Test func decodesATmdbTitle() throws {
        let json = """
        {"key":"movie:603","name":"Matrix","year":1999,"kind":"film","platforms":["netflix","autre"],
         "genres":["action","sf"],"overview":"…","poster":"https://image.tmdb.org/t/p/w342/m.jpg",
         "cast":["Keanu Reeves","Carrie-Anne Moss"],"length":"2 h 16","rating":8.2,"emoji":"💥","hue":12}
        """
        let title = try JSONDecoder().decode(Title.self, from: Data(json.utf8))
        #expect(title.platforms == [.netflix])
        #expect(title.castLine == "Avec Keanu Reeves et Carrie-Anne Moss")
        #expect(title.meta == "1999 · Film · 2 h 16")
        #expect(title.posterURL != nil)
    }

    @Test func statSummary() {
        #expect(Stat(title: "local:1", likes: 3, dislikes: 1).summary == "75 % aiment · 4 votes")
        #expect(Stat(title: "local:1", likes: 0, dislikes: 0).summary == nil)
    }
}
