//! Partagé par le serveur et le navigateur : le modèle des titres et les îlots.
//!
//! Le serveur rend les îlots en HTML (src/main.rs) ; compilé en WebAssembly, ce
//! fichier les rend interactifs dans le navigateur :
//!
//! - `picker` : la sélection de 5 titres (affiche, résumé, acteurs, année), les
//!   filtres, le bouton « Mélanger » et les votes ;
//! - `taste` : dans le pied de page, les préférences, les suggestions personnalisées et
//!   le formulaire pour proposer un titre.
//!
//! Les titres viennent de l'API du serveur (tout le catalogue des plateformes, par
//! Wikidata et Wikipédia), d'un fichier JSON sur un site statique, ou de la sélection embarquée. Les
//! goûts de la personne (`Taste`) sont gardés dans un `Store` du `localStorage`,
//! partagé par les deux îlots et les onglets.

pub mod catalog;

use catalog::{
    Genre, Kind, PICKS, Platform, Profile, Seen, Suggestion, Title, builtin, builtin_candidates,
    picks, recommend,
};
use paquets::orm::web::Store;
use paquets::prelude::*;

/// La clé du `Store` des goûts.
pub const TASTE_STORE: &str = "cineshuffle-taste-v2";

/// La raison d'une suggestion populaire.
pub const POPULAR: &str = "Populaire sur vos plateformes";

/// Un vote : vu, et aimé ou pas.
#[data]
#[derive(PartialEq)]
pub struct Vote {
    /// Le titre voté (de quoi le comparer aux autres).
    pub seen: Seen,
    /// Aimé (`true`) ou pas (`false`).
    pub liked: bool,
}

/// Un titre proposé par la personne.
#[data]
#[derive(PartialEq)]
pub struct Proposal {
    /// Le titre proposé.
    pub title: String,
    /// La plateforme.
    pub platform: Platform,
}

/// Les goûts d'une personne, gardés dans son navigateur.
#[data]
#[derive(Default, PartialEq)]
pub struct Taste {
    /// Ses votes.
    #[serde(default)]
    pub votes: Vec<Vote>,
    /// Ses plateformes (vide : toutes).
    #[serde(default)]
    pub platforms: Vec<Platform>,
    /// Ses genres préférés.
    #[serde(default)]
    pub genres: Vec<Genre>,
    /// Films, séries ou les deux (`None`).
    #[serde(default)]
    pub kind: Option<Kind>,
    /// Les titres qu'elle a proposés.
    #[serde(default)]
    pub proposals: Vec<Proposal>,
}

impl Taste {
    /// Le vote pour le titre `key`.
    pub fn vote(&self, key: &str) -> Option<bool> {
        self.votes
            .iter()
            .find(|v| v.seen.key == key)
            .map(|v| v.liked)
    }

    /// Vote pour `title` ; voter deux fois la même chose retire le vote. Renvoie le
    /// vote d'avant et le nouveau.
    pub fn toggle_vote(&mut self, title: &Title, liked: bool) -> (Option<bool>, Option<bool>) {
        let before = self.vote(&title.key);
        self.votes.retain(|v| v.seen.key != title.key);
        if before == Some(liked) {
            (before, None)
        } else {
            self.votes.push(Vote {
                seen: title.seen(),
                liked,
            });
            (before, Some(liked))
        }
    }

    /// Ajoute ou retire `item` de `list`.
    pub fn toggle<T: PartialEq>(list: &mut Vec<T>, item: T) {
        if let Some(at) = list.iter().position(|x| *x == item) {
            list.remove(at);
        } else {
            list.push(item);
        }
    }

    /// Le profil lu par le moteur de suggestions.
    pub fn profile(&self) -> Profile {
        let pick = |liked: bool| {
            self.votes
                .iter()
                .filter(|v| v.liked == liked)
                .map(|v| v.seen.clone())
                .collect()
        };
        Profile {
            liked: pick(true),
            disliked: pick(false),
            platforms: self.platforms.clone(),
            genres: self.genres.clone(),
            kind: self.kind,
        }
    }
}

/// D'où les îlots tirent leurs titres.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// L'API du serveur CineShuffle (son adresse, `""` pour le même site).
    Api(String),
    /// Un fichier JSON de titres (site statique).
    Pool(String),
    /// La sélection embarquée.
    Builtin,
}

impl Source {
    fn new(api: Option<String>, pool: Option<String>) -> Source {
        match (api, pool) {
            (Some(api), _) => Source::Api(api),
            (None, Some(pool)) => Source::Pool(pool),
            (None, None) => Source::Builtin,
        }
    }

    fn api(&self) -> Option<&str> {
        match self {
            Source::Api(base) => Some(base),
            _ => None,
        }
    }
}

/// Les props de l'îlot `picker`, en JSON.
#[data]
#[derive(Default)]
pub struct PickerProps {
    /// Les titres tirés par le serveur (la sélection du jour).
    pub picks: Vec<Title>,
    /// L'adresse de l'API (`""` : le même site), ou `None` sans serveur.
    pub api: Option<String>,
    /// L'adresse du fichier de titres d'un site statique.
    pub pool: Option<String>,
}

/// Les props de l'îlot `taste`, en JSON.
#[data]
#[derive(Default)]
pub struct TasteProps {
    /// Les suggestions de départ (sans vote ni préférence).
    pub suggestions: Vec<Suggestion>,
    /// L'adresse de l'API, ou `None` sans serveur.
    pub api: Option<String>,
    /// L'adresse du fichier de titres d'un site statique.
    pub pool: Option<String>,
}

/// Les votes de la communauté pour un titre (réponse de `GET /api/votes/stats`).
#[data]
#[derive(PartialEq)]
pub struct Stat {
    /// La clé du titre.
    pub title: String,
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

/// L'affiche d'un titre : l'image de Wikipédia, sur une affiche dessinée en 2D (dégradé,
/// anneaux animés, emoji) qui sert aussi quand il n'y a pas d'image.
pub fn poster(t: &Title) -> Element {
    div()
        .class("poster")
        .style(format!("--hue:{}", t.hue))
        .child(span().class("poster-ring").attr("aria-hidden", "true"))
        .child(
            span()
                .class("poster-ring poster-ring-2")
                .attr("aria-hidden", "true"),
        )
        .child(
            span()
                .class("poster-emoji")
                .attr("aria-hidden", "true")
                .text(t.emoji.clone()),
        )
        .child(t.poster.as_ref().map(|src| {
            img()
                .class("poster-img")
                .attr("src", src.clone())
                .attr("alt", format!("Affiche : {}", t.name))
                .attr("loading", "lazy")
                .attr("decoding", "async")
        }))
        .child(
            div()
                .class("platform-badges")
                .children(t.platforms.iter().map(|p| {
                    span()
                        .class(format!("platform-badge {}", p.slug()))
                        .text(p.label())
                })),
        )
}

/// Une carte de la sélection, avec ses boutons de vote.
fn card(
    t: Title,
    index: usize,
    taste: State<Taste>,
    stats: State<Vec<Stat>>,
    api: Option<String>,
) -> Node {
    let key = t.key.clone();
    let vote_button = {
        let t = t.clone();
        move |liked: bool| {
            let api = api.clone();
            let title = t.clone();
            let key = t.key.clone();
            let pressed_key = key.clone();
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
                    taste
                        .with(|s| s.vote(&pressed_key) == Some(liked))
                        .to_string()
                })
                .on_click(move |_| {
                    let (before, now) = update_taste(|s| s.toggle_vote(&title, liked));
                    // Les votes de la communauté n'existent qu'avec le serveur.
                    if let Some(base) = &api {
                        stats.update(|all| apply_vote(all, &key, before, now));
                        send_vote(base, &key, before, now);
                    }
                })
                .child(
                    span()
                        .class("vote-icon")
                        .text(if liked { "👍" } else { "👎" }),
                )
                .child(span().text(if liked { "Aimé" } else { "Pas aimé" }))
        }
    };
    let (liked_key, disliked_key, stat_key) = (key.clone(), key.clone(), key.clone());
    article()
        .class("card")
        .class_if("is-liked", move || {
            taste.with(|s| s.vote(&liked_key) == Some(true))
        })
        .class_if("is-disliked", move || {
            taste.with(|s| s.vote(&disliked_key) == Some(false))
        })
        .style(format!("--i:{index};--hue:{}", t.hue))
        .child(poster(&t))
        .child(
            div()
                .class("card-body")
                .child(h3().class("card-title").text(t.name.clone()))
                .child(
                    p().class("card-meta").text(t.meta()).child(
                        t.rating
                            .filter(|r| *r > 0.0)
                            .map(|r| span().class("rating").text(format!(" · ★ {r:.1}"))),
                    ),
                )
                .child(p().class("card-pitch").text(if t.overview.is_empty() {
                    "Résumé bientôt disponible.".to_owned()
                } else {
                    t.overview.clone()
                }))
                .child(t.cast_line().map(|c| p().class("card-cast").text(c)))
                .child(t.link.as_ref().map(|url| {
                    p().class("card-link").child(
                        a().href(url.clone())
                            .attr("rel", "noopener")
                            .attr("target", "_blank")
                            .text("Résumé : Wikipédia ↗"),
                    )
                }))
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
                    stats.with(|all| match all.iter().find(|s| s.title == stat_key) {
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
fn apply_vote(all: &mut Vec<Stat>, key: &str, before: Option<bool>, now: Option<bool>) {
    if !all.iter().any(|s| s.title == key) {
        all.push(Stat {
            title: key.to_owned(),
            likes: 0,
            dislikes: 0,
        });
    }
    if let Some(s) = all.iter_mut().find(|s| s.title == key) {
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
/// votes. Les props (`PickerProps`, JSON) donnent les titres du jour.
pub fn picker(props: &str) -> Node {
    let props: PickerProps = paquets::serde_json::from_str(props).unwrap_or_default();
    let source = Source::new(props.api, props.pool);
    let shown = use_state(props.picks);
    let platform = use_state(None::<Platform>);
    let kind = use_state(None::<Kind>);
    let round = use_state(0u32);
    let loading = use_state(false);
    let taste = use_taste();
    let stats = use_state(Vec::<Stat>::new());

    if let Some(base) = source.api().map(str::to_owned) {
        effect(move || load_stats(&base, stats));
    }

    let draw = {
        let source = source.clone();
        move || {
            loading.set(true);
            load_picks(
                source.clone(),
                random_seed(),
                platform.get(),
                kind.get(),
                move |titles| {
                    shown.set(titles);
                    round.update(|r| *r += 1);
                    loading.set(false);
                },
            );
        }
    };

    let platform_chip = {
        let draw = draw.clone();
        move |value: Option<Platform>| {
            let draw = draw.clone();
            let label = value.map_or("Toutes", Platform::label);
            let slug = value.map_or("all", Platform::slug);
            chip(label, move || platform.get() == value)
                .class(format!("chip-{slug}"))
                .on_click(move |_| {
                    platform.set(value);
                    draw();
                })
        }
    };
    let kind_chip = {
        let draw = draw.clone();
        move |value: Option<Kind>, label: &'static str| {
            let draw = draw.clone();
            chip(label, move || kind.get() == value).on_click(move |_| {
                kind.set(value);
                draw();
            })
        }
    };

    let deck_api = source.api().map(str::to_owned);
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
                        .child(platform_chip(None))
                        .children(Platform::ALL.into_iter().map(|p| platform_chip(Some(p)))),
                )
                .child(
                    div()
                        .class("chips")
                        .attr("role", "group")
                        .attr("aria-label", "Format")
                        .child(kind_chip(None, "Films et séries"))
                        .child(kind_chip(Some(Kind::Film), "Films"))
                        .child(kind_chip(Some(Kind::Serie), "Séries")),
                ),
        )
        .child(
            div()
                .class("deck")
                .class_if("is-loading", move || loading.get())
                .attr("aria-live", "polite")
                .attr_dyn("aria-busy", move || loading.get().to_string())
                .children_dyn(move || {
                    // Lu pour que chaque tirage recrée les cartes (et rejoue l'animation).
                    let _ = round.get();
                    let titles = shown.get();
                    if titles.is_empty() {
                        return vec![
                            p().class("deck-empty")
                                .text("Aucun titre ne correspond : essayez un autre filtre.")
                                .into(),
                        ];
                    }
                    titles
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
                        .bool_attr("disabled", move || loading.get())
                        .on_click(move |_| draw())
                        .child(span().class("dice").attr("aria-hidden", "true").text("🎲"))
                        .child(span().text_dyn(move || {
                            if loading.get() {
                                "Tirage en cours…".to_owned()
                            } else {
                                "Mélanger les 5 titres".to_owned()
                            }
                        })),
                )
                .child(p().class("vote-count").text_dyn(move || {
                    match taste.with(|t| t.votes.len()) {
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

/// Une suggestion du pied de page.
fn suggestion_item(i: usize, s: Suggestion) -> Node {
    let t = &s.title;
    li().class("suggestion")
        .style(format!("--i:{i};--hue:{}", t.hue))
        .child(match &t.poster {
            Some(src) => Node::from(
                img()
                    .class("suggestion-poster")
                    .attr("src", src.clone())
                    .attr("alt", "")
                    .attr("loading", "lazy")
                    .attr("decoding", "async"),
            ),
            None => Node::from(
                span()
                    .class("suggestion-emoji")
                    .attr("aria-hidden", "true")
                    .text(t.emoji.clone()),
            ),
        })
        .child(
            div()
                .class("suggestion-text")
                .child(strong().text(t.name.clone()))
                .child(small().text({
                    let mut parts = vec![t.platform_labels(), t.kind.label().to_owned()];
                    if let Some(year) = t.year {
                        parts.push(year.to_string());
                    }
                    parts.retain(|p| !p.is_empty());
                    parts.join(" · ")
                }))
                .child(
                    t.cast_line()
                        .map(|c| small().class("suggestion-cast").text(c)),
                )
                .child(span().class("reason").text(s.reason.clone())),
        )
        .into()
}

/// L'îlot du pied de page : les préférences, les suggestions personnalisées et le
/// formulaire pour proposer un titre.
pub fn taste(props: &str) -> Node {
    let props: TasteProps = paquets::serde_json::from_str(props).unwrap_or_default();
    let source = Source::new(props.api, props.pool);
    let initial = props.suggestions;
    let taste = use_taste();
    let suggestions = use_state(initial.clone());
    let request = use_state(0u32);
    let proposal = use_state(String::new());
    let proposal_platform = use_state("netflix".to_owned());
    let status = use_state(String::new());
    let api = source.api().map(str::to_owned);

    // Les suggestions suivent les goûts (dans le navigateur).
    {
        let source = source.clone();
        effect(move || {
            let current = taste.get();
            let id = untrack(|| request.get()) + 1;
            request.set(id);
            if current == Taste::default() {
                suggestions.set(initial.clone());
                return;
            }
            load_suggestions(source.clone(), current.profile(), move |found| {
                if untrack(|| request.get()) == id {
                    suggestions.set(found);
                }
            });
        });
    }

    let platform_chip = move |p: Platform| {
        chip(p.label(), move || taste.with(|t| t.platforms.contains(&p)))
            .on_click(move |_| update_taste(|t| Taste::toggle(&mut t.platforms, p)))
    };
    let genre_chip = move |g: Genre| {
        chip(g.label(), move || taste.with(|t| t.genres.contains(&g)))
            .on_click(move |_| update_taste(|t| Taste::toggle(&mut t.genres, g)))
    };
    let kind_chip = move |value: Option<Kind>, label: &'static str| {
        chip(label, move || taste.with(|t| t.kind == value))
            .on_click(move |_| update_taste(|t| t.kind = value))
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
                                .child(kind_chip(None, "Les deux"))
                                .child(kind_chip(Some(Kind::Film), "Films"))
                                .child(kind_chip(Some(Kind::Serie), "Séries")),
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
                            let found = suggestions.get();
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
                                .map(|(i, s)| suggestion_item(i, s))
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
                    let platform =
                        Platform::from_slug(&proposal_platform.get()).unwrap_or(Platform::Netflix);
                    update_taste(|t| {
                        t.proposals.push(Proposal {
                            title: title.clone(),
                            platform,
                        })
                    });
                    proposal.set(String::new());
                    match &api {
                        Some(base) => {
                            status.set(format!("Merci ! « {title} » est envoyé à l'équipe."));
                            send_proposal(base, &title, platform);
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

/// Le tirage sans serveur : dans `pool`, ou dans la sélection embarquée.
pub fn local_picks(
    pool: Option<&[Title]>,
    seed: u64,
    platform: Option<Platform>,
    kind: Option<Kind>,
) -> Vec<Title> {
    picks(pool.unwrap_or(builtin()), seed, platform, kind, PICKS)
}

/// Les suggestions sans serveur : dans `pool`, ou dans la sélection embarquée.
pub fn local_suggestions(pool: Option<&[Title]>, profile: &Profile) -> Vec<Suggestion> {
    match pool {
        Some(pool) => recommend(pool, profile, 4, POPULAR),
        None => recommend(
            &builtin_candidates(),
            profile,
            4,
            "Une valeur sûre du catalogue",
        ),
    }
}

/// L'adresse de `GET /api/picks`.
pub fn picks_url(base: &str, seed: u64, platform: Option<Platform>, kind: Option<Kind>) -> String {
    let mut url = format!("{base}/api/picks?seed={seed}");
    if let Some(p) = platform {
        url.push_str("&platform=");
        url.push_str(p.slug());
    }
    if let Some(k) = kind {
        url.push_str("&kind=");
        url.push_str(k.slug());
    }
    url
}

// ---------------------------------------------------------------------------
// Navigateur : hasard et requêtes vers l'API.
// ---------------------------------------------------------------------------

/// Une nouvelle graine.
#[cfg(target_arch = "wasm32")]
fn random_seed() -> u64 {
    (js_sys::Math::random() * 9_007_199_254_740_991.0) as u64
}

#[cfg(not(target_arch = "wasm32"))]
fn random_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
mod net {
    use std::cell::RefCell;

    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Headers, Request, RequestInit, Response};

    use super::Title;

    async fn send(req: Request) -> Option<String> {
        let window = web_sys::window()?;
        let res: Response = JsFuture::from(window.fetch_with_request(&req))
            .await
            .ok()?
            .dyn_into()
            .ok()?;
        if !res.ok() {
            return None;
        }
        JsFuture::from(res.text().ok()?).await.ok()?.as_string()
    }

    /// `GET url`, le corps en texte (`None` en cas d'erreur).
    pub async fn get(url: &str) -> Option<String> {
        send(Request::new_with_str(url).ok()?).await
    }

    /// `POST url` avec un corps JSON ; le corps de la réponse en texte.
    pub async fn post(url: &str, json: String) -> Option<String> {
        let init = RequestInit::new();
        init.set_method("POST");
        let headers = Headers::new().ok()?;
        headers.set("Content-Type", "application/json").ok()?;
        init.set_headers(&headers);
        init.set_body(&JsValue::from_str(&json));
        send(Request::new_with_str_and_init(url, &init).ok()?).await
    }

    thread_local! {
        /// Le fichier de titres d'un site statique, chargé une fois.
        static POOL: RefCell<Option<Vec<Title>>> = const { RefCell::new(None) };
    }

    /// Les titres du fichier `url` (chargés à la première demande).
    pub async fn pool(url: &str) -> Option<Vec<Title>> {
        if let Some(pool) = POOL.with(|p| p.borrow().clone()) {
            return Some(pool);
        }
        let json = get(url).await?;
        let titles: Vec<Title> = paquets::serde_json::from_str(&json).ok()?;
        POOL.with(|p| *p.borrow_mut() = Some(titles.clone()));
        Some(titles)
    }
}

/// Tire 5 titres à la source, puis appelle `done` (dans le navigateur).
#[cfg(target_arch = "wasm32")]
fn load_picks(
    source: Source,
    seed: u64,
    platform: Option<Platform>,
    kind: Option<Kind>,
    done: impl FnOnce(Vec<Title>) + 'static,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let found = match &source {
            Source::Api(base) => net::get(&picks_url(base, seed, platform, kind))
                .await
                .and_then(|json| paquets::serde_json::from_str::<Vec<Title>>(&json).ok()),
            Source::Pool(url) => net::pool(url)
                .await
                .map(|pool| local_picks(Some(&pool), seed, platform, kind)),
            Source::Builtin => None,
        };
        done(found.unwrap_or_else(|| local_picks(None, seed, platform, kind)));
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn load_picks(
    _source: Source,
    seed: u64,
    platform: Option<Platform>,
    kind: Option<Kind>,
    done: impl FnOnce(Vec<Title>) + 'static,
) {
    done(local_picks(None, seed, platform, kind));
}

/// Calcule les suggestions de `profile` à la source, puis appelle `done`.
#[cfg(target_arch = "wasm32")]
fn load_suggestions(
    source: Source,
    profile: Profile,
    done: impl FnOnce(Vec<Suggestion>) + 'static,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let found = match &source {
            Source::Api(base) => {
                let body = paquets::serde_json::to_string(&profile).unwrap_or_default();
                net::post(&format!("{base}/api/recommendations"), body)
                    .await
                    .and_then(|json| paquets::serde_json::from_str::<Vec<Suggestion>>(&json).ok())
            }
            Source::Pool(url) => net::pool(url)
                .await
                .map(|pool| local_suggestions(Some(&pool), &profile)),
            Source::Builtin => None,
        };
        done(found.unwrap_or_else(|| local_suggestions(None, &profile)));
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn load_suggestions(
    _source: Source,
    profile: Profile,
    done: impl FnOnce(Vec<Suggestion>) + 'static,
) {
    done(local_suggestions(None, &profile));
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
fn send_vote(base: &str, key: &str, before: Option<bool>, now: Option<bool>) {
    let url = format!("{base}/api/votes");
    let body = paquets::serde_json::json!({ "title": key, "previous": before, "liked": now });
    wasm_bindgen_futures::spawn_local(async move {
        let _ = net::post(&url, body.to_string()).await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn send_vote(_base: &str, _key: &str, _before: Option<bool>, _now: Option<bool>) {}

#[cfg(target_arch = "wasm32")]
fn send_proposal(base: &str, title: &str, platform: Platform) {
    let url = format!("{base}/api/proposals");
    let body = paquets::serde_json::json!({ "title": title, "platform": platform.slug() });
    wasm_bindgen_futures::spawn_local(async move {
        let _ = net::post(&url, body.to_string()).await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn send_proposal(_base: &str, _title: &str, _platform: Platform) {}

/// Le point d'entrée du navigateur : rend interactifs les îlots de la page.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    paquets::hydrate(&[("picker", picker), ("taste", taste)]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog::builtin_title;

    #[test]
    fn votes_toggle() {
        let title = builtin_title("local:3").unwrap();
        let mut t = Taste::default();
        assert_eq!(t.toggle_vote(title, true), (None, Some(true)));
        assert_eq!(t.vote("local:3"), Some(true));
        assert_eq!(t.toggle_vote(title, false), (Some(true), Some(false)));
        assert_eq!(t.votes.len(), 1);
        assert_eq!(t.toggle_vote(title, false), (Some(false), None));
        assert!(t.votes.is_empty());
    }

    #[test]
    fn stats_follow_the_votes() {
        let mut all = Vec::new();
        apply_vote(&mut all, "movie:1", None, Some(true));
        apply_vote(&mut all, "movie:1", Some(true), Some(false));
        assert_eq!(
            all,
            vec![Stat {
                title: "movie:1".into(),
                likes: 0,
                dislikes: 1
            }]
        );
    }

    #[test]
    fn islands_render_on_the_server() {
        let picks = local_picks(None, 1, None, None);
        let props = paquets::serde_json::to_string(&PickerProps {
            picks: picks.clone(),
            api: None,
            pool: None,
        })
        .unwrap();
        let html = picker(&props).render();
        assert!(html.contains("Mélanger les 5 titres"));
        assert_eq!(html.matches("class=\"card\"").count(), 5);
        assert!(taste("{}").render().contains("Suggestions pour vous"));
    }

    #[test]
    fn urls() {
        assert_eq!(
            picks_url("", 3, Some(Platform::PrimeVideo), Some(Kind::Serie)),
            "/api/picks?seed=3&platform=prime-video&kind=serie"
        );
    }
}
