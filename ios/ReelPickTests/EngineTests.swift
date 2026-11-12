import Foundation
import Testing
@testable import ReelPick

@MainActor
struct EngineTests {
    let catalog = Catalog.load(bundle: Bundle(for: AppModel.self))

    @Test func catalogIsBundled() {
        #expect(catalog.count == 60)
        #expect(Set(catalog.map(\.platform)) == Set(Platform.allCases))
    }

    @Test func picksAreReproducibleAndUnique() {
        let a = Engine.picks(from: catalog, seed: 42)
        #expect(a.count == 5)
        #expect(a == Engine.picks(from: catalog, seed: 42))
        #expect(Set(a.map(\.id)).count == 5)
        let disney = Engine.picks(from: catalog, seed: 7, platform: .disneyPlus, kind: .film)
        #expect(disney.allSatisfy { $0.platform == .disneyPlus && $0.kind == .film })
    }

    /// La même graine donne la même sélection que le site (src/catalog.rs).
    @Test func picksMatchTheWebsite() {
        var rng = Engine.Rng(seed: 0)
        #expect(rng.next() == 0xE220_A839_7B1D_CDAF)
        #expect(Engine.picks(from: catalog, seed: 42).map(\.id) == [48, 58, 46, 54, 29])
    }

    @Test func suggestionsFollowTheVotes() {
        let none = Engine.recommend(from: catalog, taste: Taste(), count: 4)
        #expect(none.first?.title.id == Engine.classics.first)

        var taste = Taste()
        taste.toggleVote(25, liked: true)
        taste.toggleVote(26, liked: true)
        let found = Engine.recommend(from: catalog, taste: taste, count: 3)
        #expect(found.allSatisfy { $0.title.genres.contains("policier") })
        #expect(found.first?.reason.hasPrefix("Parce que vous avez aimé") == true)
    }

    @Test func votesToggle() {
        var taste = Taste()
        #expect(taste.toggleVote(3, liked: true).now == true)
        #expect(taste.toggleVote(3, liked: false).now == false)
        #expect(taste.toggleVote(3, liked: false).now == nil)
        #expect(taste.votes.isEmpty)
    }

    @Test func statSummary() {
        #expect(Stat(title: 1, likes: 3, dislikes: 1).summary == "75 % aiment · 4 votes")
        #expect(Stat(title: 1, likes: 0, dislikes: 0).summary == nil)
    }
}
