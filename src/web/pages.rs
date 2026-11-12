//! Les pages, rendues par le serveur : la mise en page commune (en-tête, pied de page
//! complet avec les suggestions), l'accueil, le catalogue, l'application iOS et les
//! pages légales.

use cineshuffle::catalog::{Kind, Platform, Profile, Suggestion, Title};
use cineshuffle::{PickerProps, TasteProps, local_picks, poster};
use paquets::prelude::*;

use super::api::today_seed;
use super::{THEME_COLOR, absolute, api_base, is_static, link, pool_url, source};

/// D'où viennent les données (et leurs licences).
pub const ATTRIBUTION: &str = "Catalogue : Wikidata (licence CC0), titres associés à chaque plateforme (originaux \
     et distribution) ; la disponibilité en France peut varier. Résumés : Wikipédia (licence CC BY-SA), avec le lien \
     de chaque article. Affiches : illustrations des articles de Wikipédia, propriété de leurs ayants droit.";

/// Le catalogue complet est-il disponible (serveur, Wikidata chargé) ?
fn live_catalogue() -> bool {
    !is_static() && source::name() == "wikidata"
}

/// Le lien vers le catalogue d'une plateforme.
fn catalogue_link(p: Platform) -> String {
    if live_catalogue() {
        link(&format!("/catalogue/?plateforme={}", p.slug()))
    } else {
        link(&format!("/catalogue/#{}", p.slug()))
    }
}

/// La feuille de style du site (écrite dans la page, autorisée par sa CSP).
const STYLE: &str = include_str!("style.css");

/// Le dépôt du projet.
pub const REPOSITORY: &str = "https://github.com/maxlestage/CineShuffle";

/// Le logo : une bobine de film.
const LOGO: &str = r##"<svg class="logo-mark" viewBox="0 0 48 48" aria-hidden="true" focusable="false"><defs><linearGradient id="lg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff4d6d"/><stop offset=".5" stop-color="#a855f7"/><stop offset="1" stop-color="#22d3ee"/></linearGradient></defs><circle cx="24" cy="24" r="21" fill="url(#lg)"/><circle cx="24" cy="24" r="5" fill="#0b0b14"/><circle cx="24" cy="11.5" r="4.6" fill="#0b0b14"/><circle cx="24" cy="36.5" r="4.6" fill="#0b0b14"/><circle cx="11.5" cy="24" r="4.6" fill="#0b0b14"/><circle cx="36.5" cy="24" r="4.6" fill="#0b0b14"/></svg>"##;

const ICON_GITHUB: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M12 .5a11.5 11.5 0 0 0-3.64 22.41c.58.1.79-.25.79-.56v-2c-3.2.7-3.88-1.37-3.88-1.37-.52-1.33-1.28-1.69-1.28-1.69-1.04-.71.08-.7.08-.7 1.15.08 1.76 1.19 1.76 1.19 1.03 1.76 2.69 1.25 3.35.96.1-.75.4-1.25.73-1.54-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.29 1.19-3.1-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.77 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.81 1.19 1.84 1.19 3.1 0 4.42-2.7 5.39-5.26 5.68.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56A11.5 11.5 0 0 0 12 .5Z"/></svg>"#;
const ICON_APPLE: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M16.37 12.62c-.03-2.6 2.12-3.85 2.22-3.91-1.21-1.77-3.09-2.01-3.76-2.04-1.6-.16-3.12.94-3.93.94-.81 0-2.06-.92-3.39-.89-1.74.03-3.35 1.01-4.25 2.57-1.81 3.14-.46 7.79 1.3 10.34.86 1.25 1.89 2.65 3.24 2.6 1.3-.05 1.79-.84 3.36-.84 1.57 0 2.01.84 3.38.81 1.4-.02 2.29-1.27 3.14-2.53.99-1.45 1.4-2.85 1.42-2.92-.03-.01-2.72-1.04-2.73-4.13ZM13.8 4.97c.72-.87 1.2-2.07 1.07-3.27-1.03.04-2.28.69-3.02 1.55-.66.77-1.24 2-1.09 3.18 1.15.09 2.32-.58 3.04-1.46Z"/></svg>"#;
const ICON_PHONE: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" d="M8 2.5h8a2 2 0 0 1 2 2v15a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2v-15a2 2 0 0 1 2-2ZM11 18.5h2"/></svg>"#;
const ICON_UP: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" d="M6 14l6-6 6 6"/></svg>"#;

/// Le logo et le nom.
fn brand() -> Element {
    a().class("brand")
        .href(link("/"))
        .attr("aria-label", "CineShuffle, accueil")
        .child(span().class("logo").html(LOGO))
        .child(span().class("brand-name").text("CineShuffle"))
}

/// L'en-tête : le logo et la navigation (dépliable sur téléphone, sans JavaScript).
fn site_header(current: &str) -> Element {
    let items = [
        ("/#selection", "Sélection du jour"),
        ("/catalogue/", "Catalogue"),
        ("/application-ios/", "App iOS"),
        ("/a-propos/", "À propos"),
    ];
    let nav_list = move || {
        ul().children(items.iter().map(|(href, label)| {
            let item = a().href(link(href)).text(*label);
            li().child(if *href == current {
                item.attr("aria-current", "page")
            } else {
                item
            })
        }))
    };
    header().class("site-header").child(
        div()
            .class("container header-inner")
            .child(brand())
            .child(
                nav()
                    .class("nav-desktop")
                    .attr("aria-label", "Navigation principale")
                    .child(nav_list()),
            )
            .child(
                details()
                    .class("nav-mobile")
                    .child(
                        summary()
                            .attr("aria-label", "Ouvrir le menu")
                            .child(span().class("burger").attr("aria-hidden", "true")),
                    )
                    .child(
                        nav()
                            .attr("aria-label", "Navigation principale (mobile)")
                            .child(nav_list()),
                    ),
            ),
    )
}

/// Une colonne de liens du pied de page.
fn footer_column(title: &'static str, links: Vec<(String, &'static str, bool)>) -> Element {
    nav()
        .class("footer-col")
        .attr("aria-label", title)
        .child(h3().class("footer-title").text(title))
        .child(
            ul().children(links.into_iter().map(|(href, label, external)| {
                let a = a().href(href).text(label);
                li().child(if external {
                    a.attr("rel", "noopener").attr("target", "_blank")
                } else {
                    a
                })
            })),
        )
}

/// Le pied de page : les préférences et suggestions (îlot), les liens, les mentions.
fn site_footer(suggestions: Vec<Suggestion>) -> Element {
    let taste_props = paquets::serde_json::to_string(&TasteProps {
        suggestions,
        api: api_base(),
        pool: pool_url(),
    })
    .unwrap_or_else(|_| "{}".into());
    let mut app_links = vec![
        (link("/application-ios/"), "Application iOS", false),
        (
            link("/a-propos/#installer"),
            "Installer sur l'écran d'accueil",
            false,
        ),
    ];
    if !is_static() {
        app_links.push((link("/api/status"), "API JSON", false));
        app_links.push((link("/health"), "État du service", false));
    }
    app_links.push((
        format!("{REPOSITORY}/tree/master/ios"),
        "Code de l'app iOS",
        true,
    ));

    footer()
        .class("site-footer")
        .id("gouts")
        .child(
            div()
                .class("container")
                .child(
                    div()
                        .class("footer-hero")
                        .child(
                            div()
                                .class("footer-hero-text")
                                .child(p().class("eyebrow").text("Rien que pour vous"))
                                .child(h2().text("Des suggestions qui vous ressemblent"))
                                .child(p().class("muted").text(
                                    "Choisissez vos plateformes et vos genres, votez pour ce que vous avez vu : \
                                     CineShuffle compare les genres, le format et la plateforme de chaque titre \
                                     pour vous proposer les plus proches de vos goûts. Tout reste sur votre appareil.",
                                )),
                        )
                        .child(island(
                            "taste",
                            taste_props,
                            cineshuffle::taste,
                        )),
                )
                .child(
                    div()
                        .class("footer-grid")
                        .child(
                            div()
                                .class("footer-brand")
                                .child(brand())
                                .child(p().class("muted").text(
                                    "Ce soir, on regarde quoi ? 5 films et séries tirés au sort sur Netflix, \
                                     Prime Video, Disney+ et HBO Max, vos votes, et des suggestions selon vos goûts.",
                                ))
                                .child(
                                    div()
                                        .class("store-badges")
                                        .child(
                                            a().class("store-badge")
                                                .href(link("/application-ios/"))
                                                .child(span().class("badge-icon").html(ICON_APPLE))
                                                .child(
                                                    span()
                                                        .child(small().text("Bientôt sur"))
                                                        .child(strong().text("iPhone")),
                                                ),
                                        )
                                        .child(
                                            a().class("store-badge")
                                                .href(link("/a-propos/#installer"))
                                                .child(span().class("badge-icon").html(ICON_PHONE))
                                                .child(
                                                    span()
                                                        .child(small().text("Installer la"))
                                                        .child(strong().text("Web app")),
                                                ),
                                        ),
                                ),
                        )
                        .child(footer_column(
                            "Découvrir",
                            vec![
                                (link("/#selection"), "Sélection du jour", false),
                                (link("/catalogue/"), "Tout le catalogue", false),
                                (catalogue_link(Platform::Netflix), "Netflix", false),
                                (catalogue_link(Platform::PrimeVideo), "Prime Video", false),
                                (catalogue_link(Platform::DisneyPlus), "Disney+", false),
                                (catalogue_link(Platform::HboMax), "HBO Max", false),
                            ],
                        ))
                        .child(footer_column("Application", app_links))
                        .child(footer_column(
                            "CineShuffle",
                            vec![
                                (link("/a-propos/"), "Notre démarche", false),
                                (link("/a-propos/#comment"), "Comment ça marche", false),
                                (link("/accessibilite/"), "Accessibilité", false),
                                (REPOSITORY.to_owned(), "Code source", true),
                                (
                                    "https://github.com/maxlestage/Paquets".to_owned(),
                                    "Fait avec Paquets",
                                    true,
                                ),
                            ],
                        ))
                        .child(footer_column(
                            "Légal",
                            vec![
                                (link("/mentions-legales/"), "Mentions légales", false),
                                (link("/confidentialite/"), "Confidentialité", false),
                                (link("/confidentialite/#cookies"), "Cookies : aucun", false),
                                (format!("{REPOSITORY}/issues"), "Signaler un problème", true),
                            ],
                        )),
                )
                .child(
                    div()
                        .class("footer-bottom")
                        .child(
                            div()
                                .class("fine-print")
                                .child(
                                    div()
                                        .class("data-credits")
                                        .child(
                                            a().href("https://www.wikidata.org/")
                                                .attr("rel", "noopener")
                                                .attr("target", "_blank")
                                                .text("Wikidata"),
                                        )
                                        .child(
                                            a().href("https://fr.wikipedia.org/")
                                                .attr("rel", "noopener")
                                                .attr("target", "_blank")
                                                .text("Wikipédia"),
                                        ),
                                )
                                .child(p().text(ATTRIBUTION))
                                .child(p().text(
                                    "© 2026 CineShuffle · Disponibilités en France, à titre indicatif. Netflix, Prime Video, \
                                     Disney+ et HBO Max sont des marques de leurs propriétaires respectifs ; CineShuffle n'est \
                                     affilié à aucune d'elles.",
                                )),
                        )
                        .child(
                            div()
                                .class("footer-actions")
                                .child(
                                    a().class("icon-link")
                                        .href(REPOSITORY)
                                        .attr("rel", "noopener")
                                        .attr("target", "_blank")
                                        .attr("aria-label", "CineShuffle sur GitHub")
                                        .html(ICON_GITHUB),
                                )
                                .child(
                                    a().class("icon-link")
                                        .href("#haut")
                                        .attr("aria-label", "Revenir en haut de la page")
                                        .html(ICON_UP),
                                ),
                        ),
                ),
        )
}

/// Les suggestions de départ du pied de page (à calculer avant de construire la page :
/// les vues ne traversent pas un `.await`).
pub async fn footer_suggestions() -> Vec<Suggestion> {
    source::recommend(&Profile::default(), 4).await
}

/// La mise en page commune, avec les suggestions de départ du pied de page.
pub fn layout(
    title: &str,
    description: &str,
    path: &str,
    content: impl Into<Node>,
    suggestions: Vec<Suggestion>,
) -> Page {
    let full_title = if title == "CineShuffle" {
        "CineShuffle · Ce soir, on regarde quoi ?".to_owned()
    } else {
        format!("{title} · CineShuffle")
    };
    let mut seo = Seo::new()
        .site_name("CineShuffle")
        .locale("fr_FR")
        .og_type("website");
    if let Some(url) = absolute(&link(path)) {
        seo = seo.canonical(url);
    }
    if let Some(image) = absolute(&link("/og-image.png")) {
        seo = seo.image(image);
    }
    Page::new(full_title)
        .lang("fr")
        .description(description)
        .seo(seo)
        .plain()
        .style(STYLE)
        .head(format!(
            r#"<meta name="theme-color" content="{THEME_COLOR}"/><meta name="color-scheme" content="dark light"/><meta name="apple-mobile-web-app-capable" content="yes"/><meta name="apple-mobile-web-app-status-bar-style" content="black-translucent"/><meta name="apple-mobile-web-app-title" content="CineShuffle"/><link rel="icon" href="{}" type="image/svg+xml"/><link rel="apple-touch-icon" href="{}"/>"#,
            link("/favicon.svg"),
            link("/icons/apple-touch-icon.png"),
        ))
        .body(a().class("skip-link").href("#contenu").text("Aller au contenu"))
        .body(span().id("haut"))
        .body(site_header(path))
        .body(html::main().id("contenu").child(content))
        .body(site_footer(suggestions))
}

/// Le décor 2D animé de l'accueil : des formes, des étoiles et une pellicule qui défile.
fn hero_scene(cards: &[Title]) -> Element {
    div()
        .class("scene")
        .attr("aria-hidden", "true")
        .child(span().class("beam"))
        .children(
            ["🍿", "🎬", "🎟️", "⭐", "🎞️", "✨"]
                .into_iter()
                .enumerate()
                .map(|(i, e)| span().class("float").style(format!("--f:{i}")).text(e)),
        )
        .child(
            div()
                .class("fan")
                .children(cards.iter().take(3).enumerate().map(|(i, t)| {
                    div()
                        .class("fan-card")
                        .style(format!("--n:{i}"))
                        .child(poster(t))
                        .child(span().class("fan-title").text(t.name.clone()))
                })),
        )
        .child(
            div().class("strip").child(
                div()
                    .class("strip-track")
                    .children((0..16).map(|i| span().class("frame").style(format!("--k:{i}")))),
            ),
        )
}

/// `GET /` : la présentation, la sélection du jour et le fonctionnement.
pub async fn home(_req: Request) -> Page {
    let suggestions = footer_suggestions().await;
    let day = source::picks(today_seed(), None, None).await;
    let props = paquets::serde_json::to_string(&PickerProps {
        picks: day.clone(),
        api: api_base(),
        pool: pool_url(),
    })
    .unwrap_or_default();
    // Tout le catalogue (Wikidata, en direct ou par l'API qu'appelle le site statique), ou
    // le nombre de titres connus sans lui.
    let everything = live_catalogue() || (is_static() && api_base().is_some());
    let known = thousands(if live_catalogue() {
        source::size()
    } else {
        source::offline().len()
    });
    let count = if live_catalogue() {
        format!("{known} films et séries")
    } else if everything {
        "tous les films et séries".to_owned()
    } else {
        format!("{known} films et séries")
    };
    let content = fragment([
        Node::from(
            section()
                .class("hero")
                .child(
                    div()
                        .class("container hero-inner")
                        .child(
                            div()
                                .class("hero-text")
                                .child(p().class("eyebrow").text("Netflix · Prime Video · Disney+ · HBO Max"))
                                .child(
                                    h1().text("Ce soir, on regarde ")
                                        // Espace insécable : le « ? » ne part jamais seul à la ligne.
                                        .child(span().class("gradient-text nowrap").text("quoi\u{a0}?")),
                                )
                                .child(p().class("lead").text(
                                    "CineShuffle tire au sort 5 films et séries sur vos plateformes. \
                                     Votez pour ceux que vous avez vus : vos suggestions s'affinent à chaque vote.",
                                ))
                                .child(
                                    div()
                                        .class("hero-actions")
                                        .child(
                                            a().class("btn btn-primary")
                                                .href("#selection")
                                                .text("🎲 Voir la sélection"),
                                        )
                                        .child(
                                            a().class("btn btn-ghost")
                                                .href("#gouts")
                                                .text("Mes suggestions"),
                                        ),
                                )
                                .child(
                                    ul().class("hero-stats")
                                        .child(li().child(strong().text("4")).text(" plateformes"))
                                        .child(if everything && !live_catalogue() {
                                            li().child(strong().text("Tous")).text(" les films et séries")
                                        } else {
                                            li().child(strong().text(known.clone())).text(" films et séries")
                                        })
                                        .child(li().child(strong().text("0")).text(" cookie")),
                                ),
                        )
                        .child(hero_scene(&day)),
                ),
        ),
        Node::from(
            section()
                .class("section")
                .id("selection")
                .child(
                    div()
                        .class("container")
                        .child(
                            div()
                                .class("section-head")
                                .child(p().class("eyebrow").text("Tirage au sort"))
                                .child(h2().text("La sélection du jour"))
                                .child(p().class("muted").text(if everything {
                                    "Cinq titres tirés au hasard dans tout le catalogue, avec leur affiche, leur résumé \
                                     et leurs acteurs. Filtrez par plateforme ou par format, mélangez autant de fois que \
                                     vous voulez, et dites-nous ce que vous avez vu."
                                } else {
                                    "Cinq titres tirés au hasard chaque jour. Filtrez par plateforme ou par format, \
                                     mélangez autant de fois que vous voulez, et dites-nous ce que vous avez vu."
                                })),
                        )
                        .child(island("picker", props, cineshuffle::picker)),
                ),
        ),
        Node::from(
            section()
                .class("section section-alt")
                .id("comment")
                .child(
                    div()
                        .class("container")
                        .child(
                            div()
                                .class("section-head")
                                .child(p().class("eyebrow").text("En trois gestes"))
                                .child(h2().text("Comment ça marche")),
                        )
                        .child(
                            ol().class("steps")
                                .child(step("🎲", "Tirez au sort", format!("5 titres au hasard parmi {count} de Netflix, Prime Video, Disney+ et HBO Max.")))
                                .child(step("👍", "Votez", "Vu et aimé, vu et pas aimé : un appui suffit. Vos votes restent sur votre appareil."))
                                .child(step("✨", "Découvrez", "Les suggestions du pied de page se recalculent à chaque vote, selon les genres et le format.")),
                        ),
                ),
        ),
        Node::from(
            section().class("section").child(
                div()
                    .class("container")
                    .child(
                        div()
                            .class("section-head")
                            .child(p().class("eyebrow").text("Le catalogue"))
                            .child(h2().text("Quatre plateformes, tout leur catalogue")),
                    )
                    .child(div().class("platforms").children(Platform::ALL.into_iter().map(
                        |p| {
                            a().class(format!("platform-card {}", p.slug()))
                                .href(catalogue_link(p))
                                .child(strong().text(p.label()))
                                .child(span().text("Films et séries"))
                                .child(span().class("arrow").attr("aria-hidden", "true").text("→"))
                        },
                    ))),
            ),
        ),
        Node::from(ios_teaser(day.iter().skip(2).cloned().collect())),
    ]);
    layout(
        "CineShuffle",
        "5 films et séries tirés au sort sur Netflix, Prime Video, Disney+ et HBO Max, avec affiches, résumés et acteurs. Votez pour ce que vous avez vu et recevez des suggestions selon vos goûts.",
        "/",
        content,
        suggestions,
    )
}

/// `27122` → `27 122` (espace fine insécable).
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    out
}

/// Une étape de « Comment ça marche ».
fn step(icon: &'static str, title: &'static str, text: impl Into<String>) -> Element {
    li().class("step")
        .child(
            span()
                .class("step-icon")
                .attr("aria-hidden", "true")
                .text(icon),
        )
        .child(h3().text(title))
        .child(p().class("muted").text(text.into()))
}

/// L'encart de l'application iOS, avec un téléphone animé en 2D.
fn ios_teaser(shown: Vec<Title>) -> Element {
    section().class("section section-alt").child(
        div()
            .class("container split")
            .child(
                div()
                    .class("split-text")
                    .child(p().class("eyebrow").text("Swift · SwiftUI"))
                    .child(h2().text("CineShuffle sur iPhone"))
                    .child(p().class("muted").text(
                        "L'application iOS native reprend le tirage, les affiches, les acteurs, les votes et les \
                         suggestions, avec des animations fluides et le retour haptique. Elle utilise la même API que ce site.",
                    ))
                    .child(
                        a().class("btn btn-primary")
                            .href(link("/application-ios/"))
                            .text("Découvrir l'app iOS"),
                    ),
            )
            .child(
                div().class("phone").attr("aria-hidden", "true").child(
                    div()
                        .class("phone-screen")
                        .child(span().class("phone-notch"))
                        .children(shown.into_iter().take(3).enumerate().map(|(i, t)| {
                            div()
                                .class("phone-card")
                                .style(format!("--n:{i};--hue:{}", t.hue))
                                .child(match &t.poster {
                                    Some(src) => Node::from(
                                        img()
                                            .class("phone-poster")
                                            .attr("src", src.clone())
                                            .attr("alt", "")
                                            .attr("loading", "lazy"),
                                    ),
                                    None => Node::from(span().class("phone-emoji").text(t.emoji.clone())),
                                })
                                .child(
                                    span()
                                        .class("phone-lines")
                                        .child(strong().text(t.name.clone()))
                                        .child(small().text(t.platform_labels())),
                                )
                        })),
                ),
            ),
    )
}

/// Une carte du catalogue.
fn mini_card(t: &Title) -> Element {
    li().class("mini-card")
        .style(format!("--hue:{}", t.hue))
        .child(poster(t))
        .child(
            div()
                .class("mini-body")
                .child(h3().text(t.name.clone()))
                .child(
                    p().class("card-meta").text(t.meta()).child(
                        t.rating
                            .map(|r| span().class("rating").text(format!(" · ★ {r:.1}"))),
                    ),
                )
                .child(p().class("card-pitch").text(t.overview.clone()))
                .child(p().class("mini-genres").text(t.genre_labels())),
        )
}

/// Un onglet du catalogue.
fn tab(href: String, label: &'static str, current: bool, extra: &str) -> Element {
    let a = a().class(format!("chip {extra}")).href(href).text(label);
    if current {
        a.attr("aria-current", "page").attr("aria-pressed", "true")
    } else {
        a
    }
}

/// `GET /catalogue/?plateforme=netflix&type=film&page=2` : tout le catalogue d'une
/// plateforme, page par page (Wikidata), ou la sélection embarquée par plateforme.
pub async fn catalogue(req: Request) -> Page {
    let suggestions = footer_suggestions().await;
    let platform = req
        .query("plateforme")
        .and_then(|p| Platform::from_slug(&p))
        .unwrap_or(Platform::Netflix);
    let kind = req
        .query("type")
        .and_then(|k| Kind::from_slug(&k))
        .unwrap_or(Kind::Film);
    let page = req
        .query("page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(1)
        .clamp(1, 500);
    let browsed = if is_static() {
        None
    } else {
        source::browse(platform, kind, page).await
    };
    let head = div()
        .class("section-head")
        .child(
            p().class("eyebrow")
                .text("Netflix · Prime Video · Disney+ · HBO Max"),
        )
        .child(h1().text("Le catalogue"));

    let content = match browsed {
        Some((titles, pages)) => {
            let url = |p: Platform, k: Kind, n: u32| {
                link(&format!(
                    "/catalogue/?plateforme={}&type={}&page={n}",
                    p.slug(),
                    k.slug()
                ))
            };
            let pager = nav()
                .class("pager")
                .attr("aria-label", "Pages du catalogue")
                .child((page > 1).then(|| {
                    a().class("btn btn-ghost btn-small")
                        .attr("rel", "prev")
                        .href(url(platform, kind, page - 1))
                        .text("← Précédente")
                }))
                .child(
                    span()
                        .class("pager-info")
                        .text(format!("Page {page} sur {pages}")),
                )
                .child((page < pages).then(|| {
                    a().class("btn btn-ghost btn-small")
                        .attr("rel", "next")
                        .href(url(platform, kind, page + 1))
                        .text("Suivante →")
                }));
            section().class("section page-top").child(
                div()
                    .class("container")
                    .child(
                        head.child(p().class("muted").text(format!(
                            "Tous les {} que Wikidata associe à {}, les plus connus d'abord.",
                            if kind == Kind::Film {
                                "films"
                            } else {
                                "séries"
                            },
                            platform.label()
                        )))
                        .child(
                            div()
                                .class("chips")
                                .children(Platform::ALL.into_iter().map(|p| {
                                    tab(
                                        url(p, kind, 1),
                                        p.label(),
                                        p == platform,
                                        &format!("chip-{}", p.slug()),
                                    )
                                })),
                        )
                        .child(
                            div()
                                .class("chips")
                                .child(tab(
                                    url(platform, Kind::Film, 1),
                                    "Films",
                                    kind == Kind::Film,
                                    "",
                                ))
                                .child(tab(
                                    url(platform, Kind::Serie, 1),
                                    "Séries",
                                    kind == Kind::Serie,
                                    "",
                                )),
                        ),
                    )
                    .child(
                        ul().class("catalogue-grid")
                            .children(titles.iter().map(mini_card)),
                    )
                    .child(pager),
            )
        }
        None => {
            let all = source::offline();
            section().class("section page-top").child(
                div()
                    .class("container")
                    .child(
                        head.child(p().class("muted").text(
                            "Les films et séries que CineShuffle connaît hors ligne. Sur le site hébergé, le tirage et le \
                             catalogue couvrent tous les titres que Wikidata associe aux plateformes.",
                        ))
                        .child(div().class("chips").children(Platform::ALL.into_iter().map(|p| {
                            a().class(format!("chip chip-{}", p.slug()))
                                .href(format!("#{}", p.slug()))
                                .text(p.label())
                        }))),
                    )
                    .children(Platform::ALL.into_iter().map(|platform| {
                        div()
                            .class("catalogue-group")
                            .id(platform.slug())
                            .child(h2().text(platform.label()))
                            .child(
                                ul().class("catalogue-grid").children(
                                    all.iter()
                                        .filter(|t| t.platforms.contains(&platform))
                                        .map(mini_card),
                                ),
                            )
                    })),
            )
        }
    };
    layout(
        "Le catalogue",
        "Tous les films et séries de Netflix, Prime Video, Disney+ et HBO Max, avec affiches et résumés.",
        "/catalogue/",
        content,
        suggestions,
    )
}

/// `GET /application-ios/`
pub async fn ios(_req: Request) -> Page {
    let suggestions = footer_suggestions().await;
    let content = fragment([
        Node::from(text_page(
            "Swift · SwiftUI",
            "L'application iOS",
            vec![
                p().class("lead").text(
                    "CineShuffle existe aussi en application native pour iPhone, écrite en Swift avec SwiftUI. \
                     Elle partage l'API de ce site : même catalogue, mêmes votes, mêmes suggestions.",
                ),
                h2().text("Ce qu'elle fait"),
                ul().class("list")
                    .child(li().text("Le tirage de 5 titres, avec un filtre par plateforme et par format."))
                    .child(li().text("Les votes « Aimé » et « Pas aimé », gardés sur l'iPhone et envoyés à la communauté."))
                    .child(li().text("Des suggestions calculées par le serveur à partir de vos votes et préférences."))
                    .child(li().text("Des cartes animées en 2D, le retour haptique, le mode sombre et Dynamic Type.")),
                h2().text("L'installer sans ordinateur"),
                p().text(
                    "L'application est compilée et envoyée sur TestFlight par GitHub Actions, sur un Mac \
                     d'Apple dans le cloud : il suffit d'ajouter une clé de l'API App Store Connect dans les \
                     secrets du dépôt, depuis un téléphone. Toutes les étapes sont dans le README du dépôt.",
                ),
                p().child(
                    a().class("btn btn-primary")
                        .href(format!("{REPOSITORY}/tree/master/ios"))
                        .attr("rel", "noopener")
                        .attr("target", "_blank")
                        .text("Voir le code Swift"),
                ),
            ],
        )),
        Node::from(ios_teaser(local_picks(None, today_seed().wrapping_add(1), None, None))),
    ]);
    layout(
        "Application iOS",
        "CineShuffle sur iPhone : l'application native en Swift et SwiftUI.",
        "/application-ios/",
        content,
        suggestions,
    )
}

/// Une page de texte (à propos, mentions…).
fn text_page(eyebrow: &'static str, title: &'static str, body: Vec<Element>) -> Element {
    section().class("section page-top").child(
        div()
            .class("container prose")
            .child(p().class("eyebrow").text(eyebrow))
            .child(h1().text(title))
            .children(body),
    )
}

/// `GET /a-propos/`
pub async fn about(_req: Request) -> Page {
    let suggestions = footer_suggestions().await;
    let content = text_page(
        "CineShuffle",
        "Notre démarche",
        vec![
            p().class("lead").text(
                "Trop de choix tue l'envie : on passe plus de temps à parcourir les catalogues qu'à regarder. \
                 CineShuffle réduit le choix à cinq titres, tirés au sort parmi une sélection soignée.",
            ),
            h2().id("comment").text("Comment ça marche"),
            p().text(
                "Chaque jour, une graine change et donne une nouvelle sélection, la même pour tout le monde. \
                 Le bouton « Mélanger » en tire une autre, rien que pour vous. Quand vous votez, CineShuffle compare \
                 les genres (indice de Jaccard), le format et la plateforme des titres pour classer les suggestions.",
            ),
            h2().id("installer").text("Installer l'application"),
            ul().class("list")
                .child(li().text("iPhone et iPad : dans Safari, touchez Partager, puis « Sur l'écran d'accueil »."))
                .child(li().text("Android : dans Chrome, ouvrez le menu ⋮, puis « Installer l'application »."))
                .child(li().text("Ordinateur : l'icône d'installation apparaît dans la barre d'adresse.")),
            p().text("Une fois installée, CineShuffle s'ouvre en plein écran et fonctionne même sans réseau."),
            h2().text("Technique"),
            p().text(
                "Le site est écrit en Rust avec le framework Paquets : les pages sont rendues par le serveur, \
                 et les parties interactives sont des îlots compilés en WebAssembly. Les films et séries viennent \
                 de Wikidata (tous les titres associés à chaque plateforme), les résumés et les affiches de \
                 Wikipédia : des données libres, sans clé ni compte. Il est déployé sur Heroku et en version \
                 statique sur GitHub Pages.",
            ),
        ],
    );
    layout(
        "À propos",
        "Pourquoi et comment CineShuffle choisit vos films et séries.",
        "/a-propos/",
        content,
        suggestions,
    )
}

/// `GET /mentions-legales/`
pub async fn legal(_req: Request) -> Page {
    let suggestions = footer_suggestions().await;
    let content = text_page(
        "Légal",
        "Mentions légales",
        vec![
            h2().text("Éditeur"),
            p().text("CineShuffle est un projet personnel édité par Maxime Nathan Lestage."),
            p().child(text("Contact : par les "))
                .child(
                    a().href(format!("{REPOSITORY}/issues"))
                        .attr("rel", "noopener")
                        .text("tickets du dépôt GitHub"),
                )
                .child(text(".")),
            h2().text("Hébergement"),
            p().text("Le site est hébergé par Salesforce, Inc. (Heroku), 415 Mission Street, San Francisco, CA 94105, États-Unis, et en version statique par GitHub, Inc. (GitHub Pages), 88 Colin P. Kelly Jr. Street, San Francisco, CA 94107, États-Unis."),
            h2().text("Propriété intellectuelle"),
            p().text(
                "Netflix, Prime Video, Disney+ et HBO Max sont des marques de leurs propriétaires respectifs. CineShuffle n'est ni \
                 affilié, ni approuvé, ni sponsorisé par ces services.",
            ),
            h2().text("Données des films et séries"),
            p().text(ATTRIBUTION),
            p().text(
                "Chaque fiche renvoie à son article Wikipédia. Les affiches restent la propriété de leurs ayants \
                 droit. Les résumés et les affiches dessinées de la sélection embarquée sont originaux.",
            ),
        ],
    );
    layout(
        "Mentions légales",
        "Éditeur, hébergement et propriété intellectuelle de CineShuffle.",
        "/mentions-legales/",
        content,
        suggestions,
    )
}

/// `GET /confidentialite/`
pub async fn privacy(_req: Request) -> Page {
    let suggestions = footer_suggestions().await;
    let content = text_page(
        "Légal",
        "Confidentialité",
        vec![
            p().class("lead").text("CineShuffle ne collecte aucune donnée personnelle et ne vous suit pas."),
            h2().text("Ce qui reste sur votre appareil"),
            p().text(
                "Vos votes, vos plateformes, vos genres préférés et les titres que vous proposez sont gardés \
                 dans le stockage local de votre navigateur (localStorage). Le bouton « Réinitialiser mes goûts » \
                 les efface.",
            ),
            h2().text("Ce qui est envoyé au serveur"),
            p().text(
                "Sur le site hébergé, chaque vote est compté de façon anonyme (un compteur par titre, sans \
                 identifiant), et les titres proposés sont enregistrés sans aucune information sur vous.",
            ),
            h2().id("cookies").text("Cookies"),
            p().text("Aucun cookie, aucune mesure d'audience, aucune publicité."),
        ],
    );
    layout(
        "Confidentialité",
        "CineShuffle ne collecte aucune donnée personnelle : vos goûts restent sur votre appareil.",
        "/confidentialite/",
        content,
        suggestions,
    )
}

/// `GET /accessibilite/`
pub async fn accessibility(_req: Request) -> Page {
    let suggestions = footer_suggestions().await;
    let content = text_page(
        "Légal",
        "Accessibilité",
        vec![
            p().class("lead").text("CineShuffle vise le niveau AA des règles WCAG 2.2."),
            ul().class("list")
                .child(li().text("Navigation complète au clavier, avec un lien d'évitement et des focus visibles."))
                .child(li().text("Boutons de vote et filtres annoncés (aria-pressed) aux lecteurs d'écran."))
                .child(li().text("Animations coupées quand le système demande moins de mouvement."))
                .child(li().text("Contrastes suffisants en thème sombre comme en thème clair."))
                .child(li().text("Mise en page fluide de 320 pixels de large aux grands écrans, sans défilement horizontal.")),
            p().child(text("Un problème ? Dites-le-nous dans les "))
                .child(
                    a().href(format!("{REPOSITORY}/issues"))
                        .attr("rel", "noopener")
                        .text("tickets du dépôt"),
                )
                .child(text(".")),
        ],
    );
    layout(
        "Accessibilité",
        "La déclaration d'accessibilité de CineShuffle.",
        "/accessibilite/",
        content,
        suggestions,
    )
}

/// La page « introuvable ».
pub async fn not_found_page() -> Page {
    let suggestions = footer_suggestions().await;
    let content = section().class("section page-top not-found").child(
        div()
            .class("container prose center")
            .child(
                p().class("big-emoji")
                    .attr("aria-hidden", "true")
                    .text("🎞️"),
            )
            .child(h1().text("Cette page n'est pas à l'affiche"))
            .child(p().class("muted").text(
                "Elle a peut-être changé d'adresse. La sélection du jour, elle, vous attend.",
            ))
            .child(
                a().class("btn btn-primary")
                    .href(link("/"))
                    .text("Retour à l'accueil"),
            ),
    );
    layout(
        "Page introuvable",
        "Cette page n'existe pas.",
        "/404/",
        content,
        suggestions,
    )
    .status(404)
}

/// Toutes les adresses qui n'existent pas.
pub async fn not_found(_req: Request) -> Page {
    not_found_page().await
}

/// Les pages du site (sitemap et export statique).
pub const PAGES: [&str; 7] = [
    "/",
    "/catalogue/",
    "/application-ios/",
    "/a-propos/",
    "/mentions-legales/",
    "/confidentialite/",
    "/accessibilite/",
];

/// `GET /sitemap.xml`
pub async fn sitemap(_req: Request) -> Response {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    for page in PAGES {
        let url = absolute(&link(page)).unwrap_or_else(|| link(page));
        xml.push_str(&format!("<url><loc>{url}</loc></url>"));
    }
    xml.push_str("</urlset>");
    res::status(200)
        .header("content-type", "application/xml; charset=utf-8")
        .text(xml)
}

/// `GET /robots.txt`
pub async fn robots(_req: Request) -> String {
    let sitemap = absolute(&link("/sitemap.xml")).unwrap_or_else(|| link("/sitemap.xml"));
    format!("User-agent: *\nAllow: /\nDisallow: /api/\nSitemap: {sitemap}\n")
}
