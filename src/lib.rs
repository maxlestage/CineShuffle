//! Partagé par le serveur et le navigateur : le catalogue et les îlots.
//!
//! Le serveur rend les îlots en HTML (src/main.rs) ; compilé en WebAssembly, ce
//! fichier les rend interactifs dans le navigateur :
//!
//! - `picker` : la sélection de 5 titres, les filtres, le bouton « Mélanger » et les votes ;
//! - `taste` : dans le pied de page, les préférences et les suggestions personnalisées,
//!   et le formulaire pour proposer un titre.
//!
//! Les deux îlots partagent les goûts de la personne (`Taste`) dans un `Store` gardé
//! dans le `localStorage`, synchronisé entre îlots et onglets.

pub mod catalog;

use catalog::{CATALOG, Genre, Kind, PICKS, Platform, Profile, Title, picks, recommend};
use paquets::orm::web::Store;
use paquets::prelude::*;

/// La clé du `Store` des goûts.
pub const TASTE_STORE: &str = "reelpick-taste";

/// Un vote : vu, et aimé ou pas.
#[data]
#[derive(PartialEq)]
pub struct Vote {
    /// L'identifiant du titre.
    pub title: u32,
    /// Aimé (`true`) ou pas (`false`).
    pub liked: bool,
}

/// Un titre proposé par la personne et gardé sur son appareil.
#[data]
#[derive(PartialEq)]
pub struct Proposal {
    /// Le titre proposé.
    pub title: String,
    /// La plateforme (`netflix`, `hbo-max`, `disney-plus`).
    pub platform: String,
}

/// Les goûts d'une personne, gardés dans son navigateur.
#[data]
#[derive(Default, PartialEq)]
pub struct Taste {
    /// Ses votes.
    pub votes: Vec<Vote>,
    /// Ses plateformes (identifiants ; vide : toutes).
    pub platforms: Vec<String>,
    /// Ses genres préférés (identifiants).
    pub genres: Vec<String>,
    /// `film`, `serie` ou vide (les deux).
    pub kind: String,
    /// Les titres qu'elle a proposés.
    pub proposals: Vec<Proposal>,
}

impl Taste {
    /// Le vote pour `title`.
    pub fn vote(&self, title: u32) -> Option<bool> {
        self.votes
            .iter()
            .find(|v| v.title == title)
            .map(|v| v.liked)
    }

    /// Vote pour `title` ; voter deux fois la même chose retire le vote.
    pub fn toggle_vote(&mut self, title: u32, liked: bool) -> Option<bool> {
        let before = self.vote(title);
        self.votes.retain(|v| v.title != title);
        if before == Some(liked) {
            None
        } else {
            self.votes.push(Vote { title, liked });
            Some(liked)
        }
    }

    /// Ajoute ou retire `item` de `list`.
    pub fn toggle(list: &mut Vec<String>, item: &str) {
        if let Some(at) = list.iter().position(|x| x == item) {
            list.remove(at);
        } else {
            list.push(item.to_owned());
        }
    }

    /// Le profil lu par le moteur de suggestions.
    pub fn profile(&self) -> Profile {
        Profile {
            liked: self
                .votes
                .iter()
                .filter(|v| v.liked)
                .map(|v| v.title)
                .collect(),
            disliked: self
                .votes
                .iter()
                .filter(|v| !v.liked)
                .map(|v| v.title)
                .collect(),
            platforms: self
                .platforms
                .iter()
                .filter_map(|s| Platform::from_slug(s))
                .collect(),
            genres: self
                .genres
                .iter()
                .filter_map(|s| Genre::from_slug(s))
                .collect(),
            kind: Kind::from_slug(&self.kind),
        }
    }
}

/// Les props de l'îlot `picker`, en JSON.
#[data]
#[derive(Default)]
pub struct PickerProps {
    /// La graine du tirage (la sélection du jour).
    pub seed: u64,
    /// L'adresse de l'API (`""` : le même site), ou `None` sans serveur (GitHub Pages).
    pub api: Option<String>,
}

/// Les props de l'îlot `taste`, en JSON.
#[data]
#[derive(Default)]
pub struct TasteProps {
    /// L'adresse de l'API, ou `None` sans serveur.
    pub api: Option<String>,
}

/// Les votes de la communauté pour un titre (réponse de `GET /api/votes/stats`).
#[data]
#[derive(PartialEq)]
pub struct Stat {
    /// L'identifiant du titre.
    pub title: u32,
    /// Les « aimé ».
    pub likes: u32,
    /// Les « pas aimé ».
    pub dislikes: u32,
}

/// Un état qui suit le `Store` des goûts : il part de `Taste::default()` (ce que rend
/// le serveur), puis prend la valeur gardée dans le navigateur.
fn use_taste() -> State<Taste> {
    let taste = use_state(Taste::default());
    // Les effets ne tournent que dans le navigateur.
    effect(move || {
        let store = Store::<Taste>::new(TASTE_STORE);
        taste.set(store.state());
        let follow = store.subscribe_local(move |t: &Taste| taste.set(t.clone()));
        on_cleanup(move || drop(follow));
    });
    taste
}

/// Change les goûts gardés (et donc tous les îlots qui les suivent).
fn update_taste<R>(f: impl FnOnce(&mut Taste) -> R) -> R {
    Store::<Taste>::new(TASTE_STORE).update(f)
}

/// Une puce à bascule (`aria-pressed`).
fn chip(label: impl Into<String>, pressed: impl Fn() -> bool + 'static) -> Element {
    button()
        .attr("type", "button")
        .class("chip")
        .attr_dyn("aria-pressed", move || pressed().to_string())
        .text(label.into())
}

/// L'affiche 2D d'un titre : un dégradé, un emoji et des formes animées.
pub fn poster(t: &Title) -> Element {
    div()
        .class("poster")
        .attr("aria-hidden", "true")
        .style(format!("--hue:{}", t.hue))
        .child(span().class("poster-ring"))
        .child(span().class("poster-ring poster-ring-2"))
        .child(span().class("poster-emoji").text(t.emoji))
        .child(
            span()
                .class(format!("platform-badge {}", t.platform.slug()))
                .text(t.platform.label()),
        )
}

/// Une carte de la sélection, avec ses boutons de vote.
fn card(
    t: &'static Title,
    index: usize,
    taste: State<Taste>,
    stats: State<Vec<Stat>>,
    api: Option<String>,
) -> Node {
    let id = t.id;
    let vote_button = move |liked: bool| {
        let api = api.clone();
        button()
            .attr("type", "button")
            .class(if liked {
                "vote vote-up"
            } else {
                "vote vote-down"
            })
            .attr(
                "aria-label",
                format!(
                    "{} : {}",
                    if liked {
                        "Vu et aimé"
                    } else {
                        "Vu, pas aimé"
                    },
                    t.name
                ),
            )
            .attr_dyn("aria-pressed", move || {
                taste.with(|s| s.vote(id) == Some(liked)).to_string()
            })
            .on_click(move |_| {
                let before = taste.with(|s| s.vote(id));
                let now = update_taste(|s| s.toggle_vote(id, liked));
                // Les votes de la communauté n'existent qu'avec le serveur.
                if let Some(base) = &api {
                    stats.update(|all| apply_vote(all, id, before, now));
                    send_vote(base, id, before, now);
                }
            })
            .child(
                span()
                    .class("vote-icon")
                    .text(if liked { "👍" } else { "👎" }),
            )
            .child(span().text(if liked { "Aimé" } else { "Pas aimé" }))
    };
    article()
        .class("card")
        .class_if("is-liked", move || taste.with(|s| s.vote(id) == Some(true)))
        .class_if("is-disliked", move || {
            taste.with(|s| s.vote(id) == Some(false))
        })
        .style(format!("--i:{index};--hue:{}", t.hue))
        .child(poster(t))
        .child(
            div()
                .class("card-body")
                .child(h3().class("card-title").text(t.name))
                .child(p().class("card-meta").text(t.meta()))
                .child(p().class("card-pitch").text(t.pitch))
                .child(
                    ul().class("tags")
                        .attr("aria-label", "Genres")
                        .children(t.genres.iter().map(|g| li().text(g.label()))),
                ),
        )
        .child(
            div()
                .class("card-vote")
                .child(p().class("vote-question").text("Vous l'avez vu ?"))
                .child(
                    div()
                        .class("vote-buttons")
                        .child(vote_button(true))
                        .child(vote_button(false)),
                )
                .child(p().class("community").text_dyn(move || {
                    stats.with(|all| match all.iter().find(|s| s.title == id) {
                        Some(s) if s.likes + s.dislikes > 0 => {
                            let total = s.likes + s.dislikes;
                            format!(
                                "{} % aiment · {} vote{}",
                                s.likes * 100 / total,
                                total,
                                if total > 1 { "s" } else { "" }
                            )
                        }
                        _ => String::new(),
                    })
                })),
        )
        .into()
}

/// Reporte un vote dans les statistiques affichées, sans attendre le serveur.
fn apply_vote(all: &mut Vec<Stat>, id: u32, before: Option<bool>, now: Option<bool>) {
    if !all.iter().any(|s| s.title == id) {
        all.push(Stat {
            title: id,
            likes: 0,
            dislikes: 0,
        });
    }
    if let Some(s) = all.iter_mut().find(|s| s.title == id) {
        match before {
            Some(true) => s.likes = s.likes.saturating_sub(1),
            Some(false) => s.dislikes = s.dislikes.saturating_sub(1),
            None => {}
        }
        match now {
            Some(true) => s.likes += 1,
            Some(false) => s.dislikes += 1,
            None => {}
        }
    }
}

/// L'îlot de la sélection : 5 titres tirés au sort, des filtres, « Mélanger » et les
/// votes. Les props (`PickerProps`, JSON) donnent la graine du jour.
pub fn picker(props: &str) -> Node {
    let props: PickerProps = paquets::serde_json::from_str(props).unwrap_or_default();
    let seed = use_state(props.seed);
    let platform = use_state(String::new());
    let kind = use_state(String::new());
    let round = use_state(0u32);
    let taste = use_taste();
    let stats = use_state(Vec::<Stat>::new());
    let api = props.api.clone();

    if let Some(base) = api.clone() {
        effect(move || load_stats(&base, stats));
    }

    let platform_chip = move |slug: &'static str, label: &'static str| {
        chip(label, move || platform.with(|p| p == slug))
            .class(format!(
                "chip-{}",
                if slug.is_empty() { "all" } else { slug }
            ))
            .on_click(move |_| {
                platform.set(slug.to_owned());
                round.update(|r| *r += 1);
            })
    };
    let kind_chip = move |slug: &'static str, label: &'static str| {
        chip(label, move || kind.with(|k| k == slug)).on_click(move |_| {
            kind.set(slug.to_owned());
            round.update(|r| *r += 1);
        })
    };

    let deck_api = api.clone();
    section()
        .class("picker")
        .attr("aria-label", "Sélection aléatoire")
        .child(
            div()
                .class("filters")
                .child(
                    div()
                        .class("chips")
                        .attr("role", "group")
                        .attr("aria-label", "Plateforme")
                        .child(platform_chip("", "Toutes"))
                        .children(
                            Platform::ALL
                                .into_iter()
                                .map(move |p| platform_chip(p.slug(), p.label())),
                        ),
                )
                .child(
                    div()
                        .class("chips")
                        .attr("role", "group")
                        .attr("aria-label", "Format")
                        .child(kind_chip("", "Films et séries"))
                        .child(kind_chip("film", "Films"))
                        .child(kind_chip("serie", "Séries")),
                ),
        )
        .child(
            div()
                .class("deck")
                .attr("aria-live", "polite")
                .children_dyn(move || {
                    let chosen = picks(
                        seed.get(),
                        platform.with(|p| Platform::from_slug(p)),
                        kind.with(|k| Kind::from_slug(k)),
                        PICKS,
                    );
                    // Lu pour que chaque tirage recrée les cartes (et rejoue l'animation).
                    let _ = round.get();
                    chosen
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| card(t, i, taste, stats, deck_api.clone()))
                        .collect()
                }),
        )
        .child(
            div()
                .class("picker-actions")
                .child(
                    button()
                        .attr("type", "button")
                        .class("btn btn-primary shuffle")
                        .on_click(move |_| {
                            seed.set(random_seed(seed.get()));
                            round.update(|r| *r += 1);
                        })
                        .child(span().class("dice").attr("aria-hidden", "true").text("🎲"))
                        .child(span().text("Mélanger les 5 titres")),
                )
                .child(p().class("vote-count").text_dyn(move || {
                    let n = taste.with(|t| t.votes.len());
                    match n {
                        0 => {
                            "Votez pour les titres que vous avez vus : vos suggestions s'affinent."
                                .to_owned()
                        }
                        1 => "1 vote · vos suggestions s'affinent en bas de page.".to_owned(),
                        n => format!("{n} votes · vos suggestions s'affinent en bas de page."),
                    }
                })),
        )
        .into()
}

/// L'îlot du pied de page : les préférences, les suggestions personnalisées et le
/// formulaire pour proposer un titre.
pub fn taste(props: &str) -> Node {
    let props: TasteProps = paquets::serde_json::from_str(props).unwrap_or_default();
    let taste = use_taste();
    let proposal = use_state(String::new());
    let proposal_platform = use_state("netflix".to_owned());
    let status = use_state(String::new());
    let api = props.api;

    let platform_chip = move |p: Platform| {
        let slug = p.slug();
        chip(p.label(), move || {
            taste.with(|t| t.platforms.iter().any(|x| x == slug))
        })
        .on_click(move |_| update_taste(|t| Taste::toggle(&mut t.platforms, slug)))
    };
    let genre_chip = move |g: Genre| {
        let slug = g.slug();
        chip(g.label(), move || {
            taste.with(|t| t.genres.iter().any(|x| x == slug))
        })
        .on_click(move |_| update_taste(|t| Taste::toggle(&mut t.genres, slug)))
    };
    let kind_chip = move |slug: &'static str, label: &'static str| {
        chip(label, move || taste.with(|t| t.kind == slug))
            .on_click(move |_| update_taste(|t| t.kind = slug.to_owned()))
    };

    div()
        .class("taste")
        .child(
            div()
                .class("taste-prefs")
                .child(h3().class("footer-title").text("Vos préférences"))
                .child(
                    fieldset()
                        .child(legend().text("Mes plateformes"))
                        .child(
                            div()
                                .class("chips")
                                .children(Platform::ALL.into_iter().map(platform_chip)),
                        ),
                )
                .child(
                    fieldset()
                        .child(legend().text("Je préfère"))
                        .child(
                            div()
                                .class("chips")
                                .child(kind_chip("", "Les deux"))
                                .child(kind_chip("film", "Films"))
                                .child(kind_chip("serie", "Séries")),
                        ),
                )
                .child(
                    fieldset()
                        .child(legend().text("Mes genres"))
                        .child(
                            div()
                                .class("chips chips-small")
                                .children(Genre::ALL.into_iter().map(genre_chip)),
                        ),
                ),
        )
        .child(
            div()
                .class("taste-results")
                .child(h3().class("footer-title").text("Suggestions pour vous"))
                .child(
                    ol().class("suggestions")
                        .attr("aria-live", "polite")
                        .children_dyn(move || {
                            let profile = taste.with(Taste::profile);
                            let found = recommend(&profile, 4);
                            if found.is_empty() {
                                return vec![
                                    li().class("suggestion-empty")
                                        .text("Plus rien à suggérer avec ces filtres : élargissez vos plateformes ou vos genres.")
                                        .into(),
                                ];
                            }
                            found
                                .into_iter()
                                .enumerate()
                                .map(|(i, s)| {
                                    li().class("suggestion")
                                        .style(format!("--i:{i};--hue:{}", s.title.hue))
                                        .child(
                                            span()
                                                .class("suggestion-emoji")
                                                .attr("aria-hidden", "true")
                                                .text(s.title.emoji),
                                        )
                                        .child(
                                            div()
                                                .class("suggestion-text")
                                                .child(strong().text(s.title.name))
                                                .child(small().text(format!(
                                                    "{} · {} · {}",
                                                    s.title.platform.label(),
                                                    s.title.kind.label(),
                                                    s.title.year
                                                )))
                                                .child(span().class("reason").text(s.reason.text())),
                                        )
                                        .into()
                                })
                                .collect()
                        }),
                )
                .child(
                    button()
                        .attr("type", "button")
                        .class("btn btn-ghost btn-small")
                        .bool_attr("disabled", move || taste.with(|t| *t == Taste::default()))
                        .on_click(move |_| update_taste(|t| *t = Taste::default()))
                        .text("Réinitialiser mes goûts"),
                ),
        )
        .child(
            form()
                .class("propose")
                .attr("aria-label", "Proposer un titre")
                .on_submit(move |e| {
                    e.prevent_default();
                    let title = proposal.with(|p| p.trim().to_owned());
                    if title.chars().count() < 2 {
                        status.set("Indiquez un titre (2 caractères au moins).".to_owned());
                        return;
                    }
                    let platform = proposal_platform.get();
                    update_taste(|t| {
                        t.proposals.push(Proposal {
                            title: title.clone(),
                            platform: platform.clone(),
                        })
                    });
                    proposal.set(String::new());
                    match &api {
                        Some(base) => {
                            status.set(format!("Merci ! « {title} » est envoyé à l'équipe."));
                            send_proposal(base, &title, &platform);
                        }
                        None => status.set(format!(
                            "Merci ! « {title} » est gardé sur cet appareil."
                        )),
                    }
                })
                .child(h3().class("footer-title").text("Proposer un titre"))
                .child(
                    label()
                        .attr("for", "propose-title")
                        .class("sr-only")
                        .text("Titre du film ou de la série"),
                )
                .child(
                    input()
                        .id("propose-title")
                        .attr("name", "title")
                        .attr("maxlength", "120")
                        .attr("autocomplete", "off")
                        .attr("placeholder", "Un film ou une série à ajouter")
                        .bind_value(proposal),
                )
                .child(
                    div()
                        .class("propose-row")
                        .child(
                            label()
                                .attr("for", "propose-platform")
                                .class("sr-only")
                                .text("Plateforme"),
                        )
                        .child(
                            select()
                                .id("propose-platform")
                                .attr("name", "platform")
                                .bind_value(proposal_platform)
                                .children(Platform::ALL.into_iter().map(|p| {
                                    option().attr("value", p.slug()).text(p.label())
                                })),
                        )
                        .child(
                            button()
                                .attr("type", "submit")
                                .class("btn btn-primary btn-small")
                                .text("Envoyer"),
                        ),
                )
                .child(
                    p().class("propose-status")
                        .attr("role", "status")
                        .text_dyn(move || status.get()),
                ),
        )
        .into()
}

/// Le catalogue complet, pour les tests et l'API.
pub fn catalog() -> &'static [Title] {
    CATALOG
}

// ---------------------------------------------------------------------------
// Navigateur : hasard et requêtes vers l'API.
// ---------------------------------------------------------------------------

/// Une nouvelle graine, différente de la précédente.
#[cfg(target_arch = "wasm32")]
fn random_seed(previous: u64) -> u64 {
    let r = (js_sys::Math::random() * 9_007_199_254_740_991.0) as u64;
    if r == previous { r.wrapping_add(1) } else { r }
}

#[cfg(not(target_arch = "wasm32"))]
fn random_seed(previous: u64) -> u64 {
    catalog::Rng::new(previous).next_u64()
}

#[cfg(target_arch = "wasm32")]
mod net {
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Headers, Request, RequestInit, Response};

    /// `GET url`, le corps en texte (`None` en cas d'erreur).
    pub async fn get(url: &str) -> Option<String> {
        let window = web_sys::window()?;
        let res: Response = JsFuture::from(window.fetch_with_str(url))
            .await
            .ok()?
            .dyn_into()
            .ok()?;
        if !res.ok() {
            return None;
        }
        JsFuture::from(res.text().ok()?).await.ok()?.as_string()
    }

    /// `POST url` avec un corps JSON.
    pub async fn post(url: &str, json: String) -> Option<()> {
        let window = web_sys::window()?;
        let init = RequestInit::new();
        init.set_method("POST");
        let headers = Headers::new().ok()?;
        headers.set("Content-Type", "application/json").ok()?;
        init.set_headers(&headers);
        init.set_body(&JsValue::from_str(&json));
        let req = Request::new_with_str_and_init(url, &init).ok()?;
        let res: Response = JsFuture::from(window.fetch_with_request(&req))
            .await
            .ok()?
            .dyn_into()
            .ok()?;
        res.ok().then_some(())
    }
}

#[cfg(target_arch = "wasm32")]
fn load_stats(base: &str, stats: State<Vec<Stat>>) {
    let url = format!("{base}/api/votes/stats");
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(all) = net::get(&url)
            .await
            .and_then(|json| paquets::serde_json::from_str::<Vec<Stat>>(&json).ok())
        {
            stats.set(all);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn load_stats(_base: &str, _stats: State<Vec<Stat>>) {}

#[cfg(target_arch = "wasm32")]
fn send_vote(base: &str, id: u32, before: Option<bool>, now: Option<bool>) {
    let url = format!("{base}/api/votes");
    let body = paquets::serde_json::json!({ "title": id, "previous": before, "liked": now });
    wasm_bindgen_futures::spawn_local(async move {
        let _ = net::post(&url, body.to_string()).await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn send_vote(_base: &str, _id: u32, _before: Option<bool>, _now: Option<bool>) {}

#[cfg(target_arch = "wasm32")]
fn send_proposal(base: &str, title: &str, platform: &str) {
    let url = format!("{base}/api/proposals");
    let body = paquets::serde_json::json!({ "title": title, "platform": platform });
    wasm_bindgen_futures::spawn_local(async move {
        let _ = net::post(&url, body.to_string()).await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn send_proposal(_base: &str, _title: &str, _platform: &str) {}

/// Le point d'entrée du navigateur : rend interactifs les îlots de la page.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    paquets::hydrate(&[("picker", picker), ("taste", taste)]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn votes_toggle() {
        let mut t = Taste::default();
        assert_eq!(t.toggle_vote(3, true), Some(true));
        assert_eq!(t.vote(3), Some(true));
        assert_eq!(t.toggle_vote(3, false), Some(false));
        assert_eq!(t.votes.len(), 1);
        assert_eq!(t.toggle_vote(3, false), None);
        assert!(t.votes.is_empty());
    }

    #[test]
    fn stats_follow_the_votes() {
        let mut all = Vec::new();
        apply_vote(&mut all, 1, None, Some(true));
        apply_vote(&mut all, 1, Some(true), Some(false));
        assert_eq!(
            all,
            vec![Stat {
                title: 1,
                likes: 0,
                dislikes: 1
            }]
        );
    }

    #[test]
    fn islands_render_on_the_server() {
        let props = r#"{"seed":1,"api":null}"#;
        let html = picker(props).render();
        let expected = picks(1, None, None, PICKS);
        for t in expected {
            assert!(
                html.contains(t.name.replace('\'', "&#x27;").as_str()) || html.contains(t.name)
            );
        }
        assert!(taste("{}").render().contains("Suggestions pour vous"));
    }
}
