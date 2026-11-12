//! Le catalogue complet, sans clé ni compte : Wikidata et Wikipédia.
//!
//! - **Catalogue** : tous les films et séries que Wikidata associe à Netflix, Prime
//!   Video, Disney+ et HBO Max (« distribué par » ou « diffuseur d'origine »), avec leur
//!   titre français, leur année, leurs genres et leur notoriété (le nombre d'articles
//!   Wikipédia). Une requête SPARQL sur QLever (rapide), ou sur le service de Wikidata,
//!   rechargée chaque jour en arrière-plan.
//! - **Fiches** : le résumé de Wikipédia en français, l'affiche de l'article (Wikipédia
//!   en anglais, puis en français), les acteurs principaux et la durée (Wikidata),
//!   chargés à la demande et gardés en mémoire.
//!
//! Wikidata est sous licence CC0, les résumés de Wikipédia sous CC BY-SA : chaque fiche
//! garde le lien de son article.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cineshuffle::POPULAR;
use cineshuffle::catalog::{
    Genre, Kind, PICKS, Platform, Profile, Rng, Suggestion, Title, hue_for, picks, recommend,
};
use paquets::data;
use paquets::serde_json::{self, Value};

/// Le User-Agent demandé par Wikimedia : le nom de l'application et son adresse.
pub const USER_AGENT: &str = concat!(
    "CineShuffle/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/maxlestage/CineShuffle)"
);

/// Les services SPARQL, dans l'ordre : QLever répond en quelques secondes au catalogue
/// entier ; le service de Wikidata sert de secours, plateforme par plateforme.
const QLEVER: &str = "https://qlever.dev/api/wikidata";
const WDQS: &str = "https://query.wikidata.org/sparql";

/// Le catalogue est rechargé une fois par jour.
const REFRESH: Duration = Duration::from_secs(24 * 3600);
/// Après un échec, nouvel essai au plus tôt dix minutes plus tard.
const RETRY: Duration = Duration::from_secs(600);
/// Les fiches complètes sont gardées une semaine.
const DETAILS_TTL: Duration = Duration::from_secs(7 * 24 * 3600);
/// Le nombre de fiches gardées au plus.
const DETAILS_SIZE: usize = 5000;
/// Le nombre de titres par page du catalogue.
pub const PAGE_SIZE: usize = 20;

/// Les éléments Wikidata de chaque plateforme (HBO Max garde aussi les séries HBO).
fn platform_items(p: Platform) -> &'static [&'static str] {
    match p {
        Platform::Netflix => &["Q907311"],
        Platform::PrimeVideo => &["Q4740856"],
        Platform::DisneyPlus => &["Q54958752"],
        Platform::HboMax => &["Q65359104", "Q23633"],
    }
}

/// Film, film d'animation, documentaire, téléfilm.
const FILM_CLASSES: [&str; 4] = ["Q11424", "Q202866", "Q93204", "Q506240"];
/// Série télévisée, web-série, mini-série, série d'animation, série d'animation japonaise…
const SERIES_CLASSES: [&str; 6] = [
    "Q5398426",
    "Q526877",
    "Q1259759",
    "Q581714",
    "Q63952888",
    "Q117467246",
];

/// Ce qui n'a pas sa place dans un tirage de films et séries : talk-shows, téléréalité,
/// jeux télévisés, spectacles de stand-up.
const EXCLUDED_GENRES: [&str; 4] = ["Q622812", "Q182415", "Q846662", "Q145806"];

/// Les genres de CineShuffle d'un genre Wikidata.
fn genres_of(item: &str) -> &'static [Genre] {
    use Genre::*;
    match item {
        "Q130232" | "Q1366112" | "Q21010853" | "Q25372" | "Q1919632" | "Q643873" | "Q775344"
        | "Q1146335" | "Q102429885" => &[Drame],
        "Q859369" => &[Comedie, Drame],
        "Q7168625" => &[Historique, Drame],
        "Q157443" | "Q9335576" | "Q170238" | "Q7696995" | "Q40831" | "Q53094" | "Q5778924"
        | "Q459435" | "Q17113138" | "Q663106" => &[Comedie],
        "Q860626" | "Q118612349" => &[Comedie, Romance],
        "Q2678111" => &[Action, Comedie],
        "Q761469" => &[Sf, Comedie],
        "Q224700" => &[Horreur, Comedie],
        "Q4765080" => &[Animation, Comedie],
        "Q157394" | "Q98526245" | "Q132311" | "Q1957385" => &[Fantastique],
        "Q188473" | "Q343782" | "Q1033891" => &[Action],
        "Q1535153" | "Q141622854" => &[Action, Fantastique],
        "Q93204" | "Q7603925" | "Q622370" => &[Documentaire],
        "Q319221" | "Q56064758" | "Q172980" => &[Aventure],
        "Q471839" | "Q140472311" | "Q20443008" | "Q2973181" => &[Sf],
        "Q2484376" | "Q67175872" | "Q182015" | "Q590103" | "Q11304653" | "Q2297927" => &[Thriller],
        "Q19367312" | "Q2421031" => &[Crime, Thriller],
        "Q1361932" | "Q2143665" | "Q1273568" | "Q28026639" => &[Famille],
        "Q645928" | "Q17013749" | "Q20267837" | "Q369747" => &[Historique],
        "Q1054574" | "Q84270297" => &[Romance],
        "Q959790" | "Q9335577" | "Q2321734" | "Q3541116" | "Q496523" => &[Crime],
        "Q200092" | "Q20220309" | "Q1342372" | "Q31888058" => &[Horreur],
        "Q1200678" | "Q101898470" => &[Mystere],
        "Q202866" | "Q138600175" | "Q25110269" | "Q17175676" | "Q581714" | "Q63952888"
        | "Q1107" => &[Animation],
        _ => &[],
    }
}

/// La requête du catalogue, pour `platforms` et les formats `kinds`.
pub fn catalogue_query(platforms: &[Platform], kinds: &[Kind]) -> String {
    let items = platforms
        .iter()
        .flat_map(|p| platform_items(*p))
        .map(|q| format!("wd:{q}"))
        .collect::<Vec<_>>()
        .join(" ");
    let classes = kinds
        .iter()
        .flat_map(|k| match k {
            Kind::Film => &FILM_CLASSES[..],
            Kind::Serie => &SERIES_CLASSES[..],
        })
        .map(|q| format!("wd:{q}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"PREFIX wd: <http://www.wikidata.org/entity/>
PREFIX wdt: <http://www.wikidata.org/prop/direct/>
PREFIX wikibase: <http://wikiba.se/ontology#>
PREFIX schema: <http://schema.org/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?item ?sites (GROUP_CONCAT(DISTINCT STR(?p); separator=" ") AS ?platforms) (SAMPLE(?cls) AS ?class) (SAMPLE(?fr) AS ?frl) (SAMPLE(?en) AS ?enl) (MIN(?date) AS ?d) (SAMPLE(?frwiki) AS ?frw) (SAMPLE(?enwiki) AS ?enw) (GROUP_CONCAT(DISTINCT STR(?g); separator=" ") AS ?genres) WHERE {{
  VALUES ?p {{ {items} }}
  VALUES ?cls {{ {classes} }}
  ?item wdt:P750|wdt:P449 ?p .
  ?item wdt:P31 ?cls .
  ?item wikibase:sitelinks ?sites .
  FILTER(?sites > 0)
  OPTIONAL {{ ?item rdfs:label ?fr FILTER(lang(?fr) = "fr") }}
  OPTIONAL {{ ?item rdfs:label ?en FILTER(lang(?en) = "en") }}
  OPTIONAL {{ ?item wdt:P577 ?date }}
  OPTIONAL {{ ?item wdt:P580 ?date }}
  OPTIONAL {{ ?item wdt:P136 ?g }}
  OPTIONAL {{ ?frwiki schema:about ?item ; schema:isPartOf <https://fr.wikipedia.org/> }}
  OPTIONAL {{ ?enwiki schema:about ?item ; schema:isPartOf <https://en.wikipedia.org/> }}
}} GROUP BY ?item ?sites"#
    )
}

/// Un titre du catalogue et ses articles Wikipédia.
#[data]
pub struct Entry {
    /// Le titre (sans résumé ni affiche : voir `details`).
    pub title: Title,
    /// L'article de Wikipédia en français.
    #[serde(default)]
    pub fr: Option<String>,
    /// L'article de Wikipédia en anglais.
    #[serde(default)]
    pub en: Option<String>,
    /// Le nombre d'articles Wikipédia (toutes langues) : la notoriété.
    #[serde(default)]
    pub sites: u32,
}

/// Le catalogue en mémoire.
pub struct Catalogue {
    /// Tous les titres, les plus connus d'abord.
    pub titles: Vec<Title>,
    /// Ceux qui ont un article Wikipédia (le tirage se fait parmi eux).
    pub drawable: Vec<Title>,
    entries: HashMap<String, Entry>,
    /// Quand il a été construit (secondes depuis 1970).
    pub built: u64,
}

impl Catalogue {
    /// Le catalogue de ces titres.
    pub fn new(mut entries: Vec<Entry>, built: u64) -> Catalogue {
        entries.sort_by(|a, b| b.sites.cmp(&a.sites).then(a.title.key.cmp(&b.title.key)));
        let titles: Vec<Title> = entries.iter().map(|e| e.title.clone()).collect();
        let drawable = entries
            .iter()
            .filter(|e| e.fr.is_some() || e.en.is_some())
            .map(|e| e.title.clone())
            .collect();
        Catalogue {
            titles,
            drawable,
            entries: entries
                .into_iter()
                .map(|e| (e.title.key.clone(), e))
                .collect(),
            built,
        }
    }

    /// Le nombre de titres.
    pub fn len(&self) -> usize {
        self.titles.len()
    }

    /// Aucun titre ?
    pub fn is_empty(&self) -> bool {
        self.titles.is_empty()
    }

    /// Un titre par sa clé.
    pub fn get(&self, key: &str) -> Option<&Title> {
        self.entries.get(key).map(|e| &e.title)
    }

    fn entries(&self) -> Vec<&Entry> {
        self.titles
            .iter()
            .filter_map(|t| self.entries.get(&t.key))
            .collect()
    }
}

/// Le dernier élément d'une adresse Wikidata (`…/entity/Q42` → `Q42`).
fn id(uri: &str) -> &str {
    uri.rsplit('/').next().unwrap_or(uri)
}

/// Décode `%C3%A9` et `_` dans le nom d'un article.
fn decode(segment: &str) -> String {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = |c: u8| (c as char).to_digit(16);
                match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                    (Some(high), Some(low)) => {
                        out.push((high * 16 + low) as u8);
                        i += 3;
                    }
                    _ => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            b'_' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Le nom d'un article à partir de son adresse, sans la précision entre parenthèses.
fn article_name(url: &str) -> Option<String> {
    let segment = url.split("/wiki/").nth(1)?;
    let name = decode(segment);
    let name = match name.rfind(" (") {
        Some(at) if name.ends_with(')') => name[..at].to_owned(),
        _ => name,
    };
    Some(name)
}

/// Les titres d'une réponse SPARQL du catalogue.
pub fn parse(body: &Value) -> Vec<Entry> {
    let rows = body["results"]["bindings"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let text = |name: &str| row[name]["value"].as_str().map(str::to_owned);
        let Some(item) = text("item") else { continue };
        let qid = id(&item).to_owned();
        let kind = match text("class").as_deref().map(id) {
            Some(c) if FILM_CLASSES.contains(&c) => Kind::Film,
            Some(c) if SERIES_CLASSES.contains(&c) => Kind::Serie,
            _ => continue,
        };
        let genre_ids: Vec<String> = text("genres")
            .unwrap_or_default()
            .split_whitespace()
            .map(|g| id(g).to_owned())
            .collect();
        if genre_ids
            .iter()
            .any(|g| EXCLUDED_GENRES.contains(&g.as_str()))
        {
            continue;
        }
        let mut genres: Vec<Genre> = Vec::new();
        if let Some(class) = text("class") {
            genres.extend(genres_of(id(&class)));
        }
        for g in &genre_ids {
            for &genre in genres_of(g) {
                if !genres.contains(&genre) {
                    genres.push(genre);
                }
            }
        }
        let mut platforms: Vec<Platform> = Vec::new();
        for p in text("platforms").unwrap_or_default().split_whitespace() {
            let p = id(p);
            if let Some(platform) = Platform::ALL
                .into_iter()
                .find(|platform| platform_items(*platform).contains(&p))
                && !platforms.contains(&platform)
            {
                platforms.push(platform);
            }
        }
        if platforms.is_empty() {
            continue;
        }
        platforms.sort_by_key(|p| Platform::ALL.iter().position(|q| q == p));
        let fr = text("frw");
        let en = text("enw");
        let Some(name) = text("frl")
            .or_else(|| fr.as_deref().and_then(article_name))
            .or_else(|| text("enl"))
            .or_else(|| en.as_deref().and_then(article_name))
        else {
            continue;
        };
        let year = text("d")
            .and_then(|d| d.trim_start_matches(['+', '-']).get(..4).map(str::to_owned))
            .and_then(|y| y.parse().ok());
        let key = format!("wd:{qid}");
        let sites = text("sites").and_then(|s| s.parse().ok()).unwrap_or(0);
        entries.push(Entry {
            title: Title {
                emoji: genres.first().map_or("🎬", |g| g.emoji()).to_owned(),
                hue: hue_for(&key),
                key,
                name,
                year,
                kind,
                platforms,
                genres,
                overview: String::new(),
                poster: None,
                cast: Vec::new(),
                length: None,
                rating: None,
                link: None,
            },
            fr,
            en,
            sites,
        });
    }
    // Un même élément peut sortir deux fois (plusieurs classes) : la première suffit.
    let mut seen = std::collections::HashSet::new();
    entries.retain(|e| seen.insert(e.title.key.clone()));
    entries
}

/// Le client HTTP.
fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default()
    })
}

/// Encode une valeur de formulaire.
fn form_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Une requête SPARQL.
async fn sparql(endpoint: &str, query: &str, timeout: Duration) -> Result<Value, String> {
    let response = http()
        .post(endpoint)
        .timeout(timeout)
        .header("Accept", "application/sparql-results+json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("query={}", form_encode(query)))
        .send()
        .await
        .map_err(|e| format!("SPARQL {endpoint} : {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("SPARQL {endpoint} : {status}"));
    }
    response
        .json()
        .await
        .map_err(|e| format!("SPARQL {endpoint} : {e}"))
}

/// Charge le catalogue complet : tout d'un coup sur QLever, sinon plateforme par
/// plateforme et format par format sur le service de Wikidata.
pub async fn fetch() -> Result<Catalogue, String> {
    let built = now();
    let both = [Kind::Film, Kind::Serie];
    match sparql(
        QLEVER,
        &catalogue_query(&Platform::ALL, &both),
        Duration::from_secs(120),
    )
    .await
    {
        Ok(body) => {
            let entries = parse(&body);
            if !entries.is_empty() {
                return Ok(Catalogue::new(entries, built));
            }
        }
        Err(e) => eprintln!("{e}"),
    }
    let mut all: Vec<Entry> = Vec::new();
    let parts = Platform::ALL
        .into_iter()
        .flat_map(|p| both.into_iter().map(move |k| (p, k)));
    for (p, kind) in parts {
        match sparql(
            WDQS,
            &catalogue_query(&[p], &[kind]),
            Duration::from_secs(70),
        )
        .await
        {
            Ok(body) => {
                for e in parse(&body) {
                    match all.iter_mut().find(|known| known.title.key == e.title.key) {
                        Some(known) => {
                            for platform in e.title.platforms {
                                if !known.title.platforms.contains(&platform) {
                                    known.title.platforms.push(platform);
                                }
                            }
                        }
                        None => all.push(e),
                    }
                }
            }
            Err(e) => eprintln!("{e}"),
        }
    }
    if all.is_empty() {
        Err("Wikidata ne répond pas".to_owned())
    } else {
        Ok(Catalogue::new(all, built))
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Le catalogue gardé dans un fichier (`cineshuffle catalogue`, au déploiement).
#[data]
struct Stored {
    built: u64,
    entries: Vec<Entry>,
}

/// Écrit le catalogue dans `path`.
pub fn save(catalogue: &Catalogue, path: &Path) -> std::io::Result<()> {
    let stored = Stored {
        built: catalogue.built,
        entries: catalogue.entries().into_iter().cloned().collect(),
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(
        path,
        serde_json::to_vec(&stored).map_err(std::io::Error::other)?,
    )
}

/// Lit le catalogue de `path`.
pub fn load(path: &Path) -> Option<Catalogue> {
    let bytes = std::fs::read(path).ok()?;
    let stored: Stored = serde_json::from_slice(&bytes).ok()?;
    (!stored.entries.is_empty()).then(|| Catalogue::new(stored.entries, stored.built))
}

static CATALOGUE: RwLock<Option<Arc<Catalogue>>> = RwLock::new(None);
static REFRESHING: AtomicBool = AtomicBool::new(false);
static LAST_TRY: Mutex<Option<Instant>> = Mutex::new(None);

/// Installe un catalogue.
pub fn install(catalogue: Catalogue) -> Arc<Catalogue> {
    let catalogue = Arc::new(catalogue);
    if let Ok(mut slot) = CATALOGUE.write() {
        *slot = Some(catalogue.clone());
    }
    catalogue
}

/// Le fichier du catalogue (`CATALOGUE_FILE`, `data/catalogue.json` par défaut).
pub fn default_file() -> std::path::PathBuf {
    std::env::var("CATALOGUE_FILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("data/catalogue.json"))
}

/// Au démarrage : le catalogue du fichier, s'il existe.
pub fn load_default_file() {
    // Les tests du serveur restent sur la sélection embarquée, sans réseau.
    if cfg!(test) {
        return;
    }
    if let Some(catalogue) = load(&default_file()) {
        println!("   catalogue : {} titres (Wikidata)", catalogue.len());
        install(catalogue);
    }
}

/// Le catalogue en mémoire ; s'il manque ou date de plus d'un jour, il est rechargé en
/// arrière-plan (le site continue avec ce qu'il a, ou la sélection embarquée).
pub fn current() -> Option<Arc<Catalogue>> {
    let catalogue = CATALOGUE.read().ok().and_then(|c| c.clone());
    let stale = catalogue
        .as_ref()
        .is_none_or(|c| now().saturating_sub(c.built) > REFRESH.as_secs());
    if stale {
        refresh_in_background();
    }
    catalogue
}

fn refresh_in_background() {
    // Les tests du serveur n'appellent pas Wikidata.
    if cfg!(test) || std::env::var_os("CINESHUFFLE_OFFLINE").is_some() {
        return;
    }
    {
        let Ok(mut last) = LAST_TRY.lock() else {
            return;
        };
        if last.is_some_and(|at| at.elapsed() < RETRY) {
            return;
        }
        *last = Some(Instant::now());
    }
    if REFRESHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        REFRESHING.store(false, Ordering::SeqCst);
        return;
    };
    runtime.spawn(async {
        match fetch().await {
            Ok(catalogue) => {
                println!("catalogue Wikidata : {} titres", catalogue.len());
                let _ = save(&catalogue, &default_file());
                install(catalogue);
            }
            Err(e) => eprintln!("{e}"),
        }
        REFRESHING.store(false, Ordering::SeqCst);
    });
}

// ---------------------------------------------------------------------------
// Les fiches : résumé, affiche, acteurs, durée.
// ---------------------------------------------------------------------------

static DETAILS: Mutex<Option<HashMap<String, (Instant, Title)>>> = Mutex::new(None);

fn cached(key: &str, full: bool) -> Option<Title> {
    let details = DETAILS.lock().ok()?;
    let (at, title) = details.as_ref()?.get(key)?;
    (at.elapsed() < DETAILS_TTL && (!full || !title.cast.is_empty() || title.length.is_some()))
        .then(|| title.clone())
}

fn remember(title: &Title) {
    if let Ok(mut details) = DETAILS.lock() {
        let map = details.get_or_insert_with(HashMap::new);
        if map.len() >= DETAILS_SIZE {
            map.clear();
        }
        map.insert(title.key.clone(), (Instant::now(), title.clone()));
    }
}

/// Le résumé d'un article Wikipédia (`/page/summary`).
#[derive(Default)]
struct Summary {
    extract: Option<String>,
    image: Option<String>,
    page: Option<String>,
}

/// `GET url` en JSON, avec un nouvel essai si le service demande de ralentir (429) ou
/// a une panne passagère. `Err` : une erreur temporaire ; `Ok(None)` : la page n'existe pas.
async fn get_json(url: &str) -> Result<Option<Value>, ()> {
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(400 * attempt)).await;
        }
        let Ok(response) = http().get(url).send().await else {
            continue;
        };
        let status = response.status();
        if status.as_u16() == 429 || status.is_server_error() {
            continue;
        }
        if !status.is_success() {
            return Ok(None);
        }
        return response.json::<Value>().await.map(Some).map_err(|_| ());
    }
    Err(())
}

/// Le résumé d'un article (`Err` : erreur temporaire, à ne pas garder en mémoire).
async fn summary(article: &str) -> Result<Summary, ()> {
    let Some((host, segment)) = article
        .strip_prefix("https://")
        .and_then(|rest| rest.split_once("/wiki/"))
    else {
        return Ok(Summary::default());
    };
    let url = format!("https://{host}/api/rest_v1/page/summary/{segment}");
    let Some(v) = get_json(&url).await? else {
        return Ok(Summary::default());
    };
    if v["type"].as_str() == Some("disambiguation") {
        return Ok(Summary::default());
    }
    Ok(Summary {
        extract: v["extract"]
            .as_str()
            .map(str::trim)
            .filter(|e| !e.is_empty())
            .map(str::to_owned),
        image: v["thumbnail"]["source"]
            .as_str()
            .or_else(|| v["originalimage"]["source"].as_str())
            .map(str::to_owned),
        page: v["content_urls"]["desktop"]["page"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| Some(article.to_owned())),
    })
}

/// Les acteurs principaux (dans l'ordre de Wikidata) et la durée ou le nombre de saisons.
async fn credits(qid: &str, kind: Kind) -> Result<(Vec<String>, Option<String>), ()> {
    let url = format!("https://www.wikidata.org/wiki/Special:EntityData/{qid}.json");
    let Some(v) = get_json(&url).await? else {
        return Ok((Vec::new(), None));
    };
    let claims = &v["entities"][qid]["claims"];
    let amount = |property: &str| {
        claims[property][0]["mainsnak"]["datavalue"]["value"]["amount"]
            .as_str()
            .and_then(|a| a.trim_start_matches('+').parse::<f64>().ok())
            .map(|a| {
                let unit = claims[property][0]["mainsnak"]["datavalue"]["value"]["unit"]
                    .as_str()
                    .map(id)
                    .unwrap_or("");
                (a, unit.to_owned())
            })
    };
    let length = match kind {
        Kind::Film => amount("P2047").and_then(|(a, unit)| {
            // Minutes (Q7727), secondes (Q11574) ou heures (Q25235).
            let minutes = match unit.as_str() {
                "Q11574" => a / 60.0,
                "Q25235" => a * 60.0,
                _ => a,
            }
            .round() as u64;
            (minutes > 0).then(|| format!("{} h {:02}", minutes / 60, minutes % 60))
        }),
        Kind::Serie => amount("P2437")
            .map(|(a, _)| a as u64)
            .filter(|s| *s > 0)
            .map(|s| {
                if s == 1 {
                    "1 saison".to_owned()
                } else {
                    format!("{s} saisons")
                }
            }),
    };
    let actors: Vec<String> = claims["P161"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(|c| c["mainsnak"]["datavalue"]["value"]["id"].as_str())
                .take(5)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    if actors.is_empty() {
        return Ok((Vec::new(), length));
    }
    let values = actors
        .iter()
        .map(|a| format!("wd:{a}"))
        .collect::<Vec<_>>()
        .join(" ");
    let query = format!(
        r#"PREFIX wd: <http://www.wikidata.org/entity/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?a ?l WHERE {{ VALUES ?a {{ {values} }} ?a rdfs:label ?l FILTER(lang(?l) = "fr" || lang(?l) = "en") }}"#
    );
    let mut names: HashMap<String, (bool, String)> = HashMap::new();
    for endpoint in [WDQS, QLEVER] {
        if let Ok(body) = sparql(endpoint, &query, Duration::from_secs(15)).await {
            for row in body["results"]["bindings"].as_array().into_iter().flatten() {
                let (Some(a), Some(l)) = (row["a"]["value"].as_str(), row["l"]["value"].as_str())
                else {
                    continue;
                };
                let french = row["l"]["xml:lang"].as_str() == Some("fr");
                let slot = names
                    .entry(id(a).to_owned())
                    .or_insert((french, l.to_owned()));
                if french && !slot.0 {
                    *slot = (true, l.to_owned());
                }
            }
            break;
        }
    }
    if names.is_empty() {
        return Err(());
    }
    let cast = actors
        .iter()
        .filter_map(|a| names.get(a).map(|(_, l)| l.clone()))
        .collect();
    Ok((cast, length))
}

/// Une fiche : le résumé et l'affiche de Wikipédia et, si `full`, les acteurs et la durée.
pub async fn details(catalogue: &Catalogue, title: &Title, full: bool) -> Title {
    if let Some(known) = cached(&title.key, full) {
        return known;
    }
    let Some(entry) = catalogue.entries.get(&title.key) else {
        return title.clone();
    };
    let qid = title.key.trim_start_matches("wd:").to_owned();
    let fr = async {
        match &entry.fr {
            Some(url) => summary(url).await,
            None => Ok(Summary::default()),
        }
    };
    let en = async {
        match &entry.en {
            Some(url) => summary(url).await,
            None => Ok(Summary::default()),
        }
    };
    let people = async {
        if full {
            credits(&qid, title.kind).await
        } else {
            Ok((Vec::new(), None))
        }
    };
    let (fr, en, people) = tokio::join!(fr, en, people);
    // Une erreur passagère : la fiche est rendue telle quelle, mais pas gardée.
    let complete = fr.is_ok() && en.is_ok() && people.is_ok();
    let (fr, en) = (fr.unwrap_or_default(), en.unwrap_or_default());
    let (cast, length) = people.unwrap_or_default();
    let mut full_title = title.clone();
    full_title.overview = fr.extract.or(en.extract).unwrap_or_default();
    // L'affiche est sur Wikipédia en anglais ; l'image de l'article en français est
    // souvent une photo (d'un acteur, du réalisateur) : elle n'est pas gardée.
    full_title.poster = en.image;
    full_title.link = fr.page.or(en.page);
    if full {
        full_title.cast = cast;
        full_title.length = length;
    }
    if complete {
        remember(&full_title);
    }
    full_title
}

/// Complète des titres en parallèle (quatre à la fois, pour ménager Wikipédia).
pub async fn complete(catalogue: &Arc<Catalogue>, titles: Vec<Title>, full: bool) -> Vec<Title> {
    let gate = Arc::new(tokio::sync::Semaphore::new(4));
    let mut set = tokio::task::JoinSet::new();
    for (i, t) in titles.iter().cloned().enumerate() {
        let catalogue = catalogue.clone();
        let gate = gate.clone();
        set.spawn(async move {
            let _permit = gate.acquire_owned().await;
            (i, details(&catalogue, &t, full).await)
        });
    }
    let mut done = titles;
    while let Some(Ok((i, t))) = set.join_next().await {
        done[i] = t;
    }
    done
}

/// 5 titres tirés au sort dans tout le catalogue (parmi ceux qui ont un article).
///
/// Sur toutes les plateformes, chaque titre est d'abord tiré d'une plateforme au hasard
/// (sinon Netflix, de loin la plus grande sur Wikidata, prendrait presque toute la
/// place). Les titres les plus connus (le plus d'articles Wikipédia) ont plus de chances
/// de sortir, sans que les autres soient exclus.
pub async fn draw(
    catalogue: &Arc<Catalogue>,
    seed: u64,
    platform: Option<Platform>,
    kind: Option<Kind>,
) -> Vec<Title> {
    let chosen = weighted_picks(catalogue, seed, platform, kind);
    complete(catalogue, chosen, true).await
}

/// Le tirage pondéré de `draw` (reproductible pour une même graine).
pub fn weighted_picks(
    catalogue: &Catalogue,
    seed: u64,
    platform: Option<Platform>,
    kind: Option<Kind>,
) -> Vec<Title> {
    let mut rng = Rng::new(seed ^ 0x00D1_CE5E_ED00);
    // Les plateformes des 5 titres : toujours la même, ou les quatre dans un ordre au
    // hasard et une cinquième au hasard.
    let mut targets: Vec<Option<Platform>> = match platform {
        Some(p) => vec![Some(p); PICKS],
        None => {
            let mut all: Vec<Option<Platform>> = Platform::ALL.into_iter().map(Some).collect();
            for i in (1..all.len()).rev() {
                let j = rng.below(i + 1);
                all.swap(i, j);
            }
            let extra = all[rng.below(all.len())];
            all.push(extra);
            all
        }
    };
    targets.truncate(PICKS);
    let mut chosen: Vec<Title> = Vec::new();
    for target in targets {
        let candidates: Vec<(&Title, f64)> = catalogue
            .drawable
            .iter()
            .filter(|t| target.is_none_or(|p| t.platforms.contains(&p)))
            .filter(|t| kind.is_none_or(|k| t.kind == k))
            .filter(|t| !chosen.iter().any(|c| c.key == t.key))
            .map(|t| {
                let sites = catalogue.entries.get(&t.key).map_or(1, |e| e.sites.max(1));
                (t, f64::from(sites).powf(0.8))
            })
            .collect();
        let total: f64 = candidates.iter().map(|c| c.1).sum();
        if candidates.is_empty() || total <= 0.0 {
            continue;
        }
        let mut ticket = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64 * total;
        let mut pick = candidates[candidates.len() - 1].0;
        for (t, w) in &candidates {
            if ticket < *w {
                pick = t;
                break;
            }
            ticket -= w;
        }
        chosen.push(pick.clone());
    }
    // Une plateforme sans titre de ce format : on complète au hasard.
    if chosen.len() < PICKS {
        for t in picks(&catalogue.drawable, seed, platform, kind, PICKS * 2) {
            if chosen.len() >= PICKS {
                break;
            }
            if !chosen.iter().any(|c| c.key == t.key) {
                chosen.push(t);
            }
        }
    }
    chosen
}

/// Les suggestions d'un profil, dans tout le catalogue.
pub async fn suggest(catalogue: &Arc<Catalogue>, profile: &Profile, n: usize) -> Vec<Suggestion> {
    let found = recommend(&catalogue.drawable, profile, n, POPULAR);
    let titles = complete(
        catalogue,
        found.iter().map(|s| s.title.clone()).collect(),
        true,
    )
    .await;
    found
        .into_iter()
        .zip(titles)
        .map(|(s, title)| Suggestion { title, ..s })
        .collect()
}

/// Une page du catalogue d'une plateforme (les plus connus d'abord) et le nombre de pages.
pub async fn browse(
    catalogue: &Arc<Catalogue>,
    platform: Platform,
    kind: Kind,
    page: u32,
) -> (Vec<Title>, u32) {
    let all: Vec<&Title> = catalogue
        .titles
        .iter()
        .filter(|t| t.platforms.contains(&platform) && t.kind == kind)
        .collect();
    let pages = all.len().div_ceil(PAGE_SIZE).max(1) as u32;
    let page = page.clamp(1, pages);
    let titles: Vec<Title> = all
        .into_iter()
        .skip((page as usize - 1) * PAGE_SIZE)
        .take(PAGE_SIZE)
        .cloned()
        .collect();
    (complete(catalogue, titles, false).await, pages)
}

/// Les titres les plus connus de chaque plateforme, complétés (instantané d'un site
/// statique).
pub async fn snapshot(catalogue: &Arc<Catalogue>, per_list: usize) -> Vec<Title> {
    let mut chosen: Vec<Title> = Vec::new();
    for platform in Platform::ALL {
        for kind in [Kind::Film, Kind::Serie] {
            for t in catalogue
                .drawable
                .iter()
                .filter(|t| t.platforms.contains(&platform) && t.kind == kind)
                .take(per_list)
            {
                if !chosen.iter().any(|c| c.key == t.key) {
                    chosen.push(t.clone());
                }
            }
        }
    }
    complete(catalogue, chosen, false).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use paquets::serde_json::json;

    fn row(item: &str, class: &str, platforms: &str, extra: Value) -> Value {
        let mut row = json!({
            "item": { "value": format!("http://www.wikidata.org/entity/{item}") },
            "sites": { "value": "42" },
            "class": { "value": format!("http://www.wikidata.org/entity/{class}") },
            "platforms": { "value": platforms },
        });
        for (k, v) in extra.as_object().unwrap() {
            row[k] = v.clone();
        }
        row
    }

    #[test]
    fn parses_the_catalogue() {
        let body = json!({ "results": { "bindings": [
            row("Q41483", "Q11424", "http://www.wikidata.org/entity/Q907311", json!({
                "frl": { "value": "Le Bon, la Brute et le Truand" },
                "d": { "value": "1966-12-23T00:00:00Z" },
                "frw": { "value": "https://fr.wikipedia.org/wiki/Le_Bon,_la_Brute_et_le_Truand" },
                "enw": { "value": "https://en.wikipedia.org/wiki/The_Good,_the_Bad_and_the_Ugly" },
                "genres": { "value": "http://www.wikidata.org/entity/Q172980 http://www.wikidata.org/entity/Q130232" },
            })),
            // HBO compte pour HBO Max ; sans titre français, le nom de l'article sert.
            row("Q23572", "Q5398426", "http://www.wikidata.org/entity/Q23633 http://www.wikidata.org/entity/Q4740856", json!({
                "enl": { "value": "Game of Thrones" },
                "frw": { "value": "https://fr.wikipedia.org/wiki/Le_Tr%C3%B4ne_de_fer_(s%C3%A9rie)" },
                "d": { "value": "2011-04-17T00:00:00Z" },
                "genres": { "value": "http://www.wikidata.org/entity/Q98526245" },
            })),
            // Un talk-show n'est pas gardé.
            row("Q1", "Q5398426", "http://www.wikidata.org/entity/Q907311", json!({
                "frl": { "value": "Un talk-show" },
                "genres": { "value": "http://www.wikidata.org/entity/Q622812" },
            })),
            // Une plateforme inconnue non plus.
            row("Q2", "Q11424", "http://www.wikidata.org/entity/Q999", json!({ "frl": { "value": "X" } })),
        ] } });
        let entries = parse(&body);
        assert_eq!(entries.len(), 2);
        let good = &entries[0].title;
        assert_eq!(good.key, "wd:Q41483");
        assert_eq!(good.name, "Le Bon, la Brute et le Truand");
        assert_eq!(good.year, Some(1966));
        assert_eq!(good.kind, Kind::Film);
        assert_eq!(good.platforms, [Platform::Netflix]);
        assert_eq!(good.genres, [Genre::Aventure, Genre::Drame]);
        let got = &entries[1].title;
        assert_eq!(got.name, "Le Trône de fer");
        assert_eq!(got.kind, Kind::Serie);
        assert_eq!(got.platforms, [Platform::PrimeVideo, Platform::HboMax]);
        assert_eq!(got.genres, [Genre::Fantastique]);

        let catalogue = Catalogue::new(entries, 0);
        assert_eq!(catalogue.drawable.len(), 2);
        assert!(catalogue.get("wd:Q23572").is_some());
    }

    #[test]
    fn draws_spread_over_platforms() {
        let mut bindings = Vec::new();
        for (i, platform) in ["Q907311", "Q4740856", "Q54958752", "Q65359104"]
            .iter()
            .enumerate()
        {
            // Netflix a beaucoup plus de titres que les autres.
            let count = if i == 0 { 200 } else { 3 };
            for n in 0..count {
                bindings.push(row(
                    &format!("Q{}", 1000 * (i + 1) + n),
                    "Q11424",
                    &format!("http://www.wikidata.org/entity/{platform}"),
                    json!({
                        "frl": { "value": format!("Film {i}-{n}") },
                        "frw": { "value": format!("https://fr.wikipedia.org/wiki/Film_{i}_{n}") },
                    }),
                ));
            }
        }
        let catalogue = Catalogue::new(parse(&json!({ "results": { "bindings": bindings } })), 0);
        let picks = weighted_picks(&catalogue, 42, None, None);
        assert_eq!(picks.len(), 5);
        for p in Platform::ALL {
            assert!(
                picks.iter().any(|t| t.platforms.contains(&p)),
                "{p} est tirée"
            );
        }
        assert_eq!(picks, weighted_picks(&catalogue, 42, None, None));
        let prime = weighted_picks(&catalogue, 7, Some(Platform::PrimeVideo), None);
        assert_eq!(prime.len(), 3, "seulement trois films Prime Video");
        assert!(prime.iter().all(|t| t.platforms == [Platform::PrimeVideo]));
    }

    #[test]
    fn names_and_encoding() {
        assert_eq!(decode("Le_Tr%C3%B4ne_de_fer"), "Le Trône de fer");
        assert_eq!(decode("100%_pur"), "100% pur");
        assert_eq!(
            article_name("https://fr.wikipedia.org/wiki/Matrix_(film)").as_deref(),
            Some("Matrix")
        );
        assert_eq!(form_encode("a b=é"), "a+b%3D%C3%A9");
        let query = catalogue_query(&[Platform::HboMax], &[Kind::Serie]);
        assert!(query.contains("wd:Q65359104 wd:Q23633"));
        assert!(query.contains("wd:Q5398426"));
        assert!(!query.contains("wd:Q11424"));
    }

    #[test]
    fn a_stored_catalogue_round_trips() {
        let body = json!({ "results": { "bindings": [
            row("Q41483", "Q11424", "http://www.wikidata.org/entity/Q907311", json!({
                "frl": { "value": "Le Bon, la Brute et le Truand" },
                "frw": { "value": "https://fr.wikipedia.org/wiki/Le_Bon,_la_Brute_et_le_Truand" },
            })),
        ] } });
        let catalogue = Catalogue::new(parse(&body), 7);
        let file = std::env::temp_dir().join(format!("cineshuffle-{}.json", std::process::id()));
        save(&catalogue, &file).unwrap();
        let back = load(&file).unwrap();
        std::fs::remove_file(&file).ok();
        assert_eq!(back.built, 7);
        assert_eq!(back.titles, catalogue.titles);
    }

    /// Wikidata et Wikipédia en direct : `cargo test -- --ignored live`.
    #[tokio::test]
    #[ignore = "appelle Wikidata et Wikipédia"]
    async fn live() {
        let catalogue = Arc::new(fetch().await.expect("Wikidata"));
        println!(
            "{} titres, dont {} avec un article",
            catalogue.len(),
            catalogue.drawable.len()
        );
        assert!(catalogue.len() > 5000);
        for p in Platform::ALL {
            let n = catalogue
                .titles
                .iter()
                .filter(|t| t.platforms.contains(&p))
                .count();
            println!("{p} : {n}");
            assert!(n > 100, "{p}");
        }
        let picks = draw(&catalogue, 42, None, None).await;
        assert_eq!(picks.len(), 5);
        for t in &picks {
            println!(
                "{} ({:?}) {:?} {:?} {:?} — {} — {}",
                t.name,
                t.year,
                t.platforms,
                t.length,
                t.cast,
                t.poster.as_deref().unwrap_or("-"),
                t.overview.chars().take(80).collect::<String>()
            );
        }
        assert!(picks.iter().any(|t| !t.overview.is_empty()));
    }
}
