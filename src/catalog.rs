//! Le modèle des titres, le tirage au sort et le moteur de suggestions.
//!
//! Compilé deux fois : dans le serveur et dans le paquet WebAssembly des îlots. Les
//! titres viennent de TMDB (tout le catalogue des plateformes, avec affiches, résumés et
//! acteurs : voir `src/web/tmdb.rs`), ou, sans clé TMDB, de la sélection embarquée de
//! 80 titres ci-dessous, qui sert aussi de secours hors ligne.

use std::fmt;
use std::sync::OnceLock;

use paquets::data;
use paquets::serde::{Deserialize, Serialize};

/// Une plateforme de streaming.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(crate = "paquets::serde")]
pub enum Platform {
    /// Netflix.
    #[serde(rename = "netflix")]
    Netflix,
    /// Prime Video.
    #[serde(rename = "prime-video")]
    PrimeVideo,
    /// Disney+.
    #[serde(rename = "disney-plus")]
    DisneyPlus,
    /// HBO Max.
    #[serde(rename = "hbo-max")]
    HboMax,
}

impl Platform {
    /// Toutes les plateformes, dans l'ordre d'affichage.
    pub const ALL: [Platform; 4] = [
        Platform::Netflix,
        Platform::PrimeVideo,
        Platform::DisneyPlus,
        Platform::HboMax,
    ];

    /// L'identifiant dans les URL et le JSON.
    pub fn slug(self) -> &'static str {
        match self {
            Platform::Netflix => "netflix",
            Platform::PrimeVideo => "prime-video",
            Platform::DisneyPlus => "disney-plus",
            Platform::HboMax => "hbo-max",
        }
    }

    /// Le nom affiché.
    pub fn label(self) -> &'static str {
        match self {
            Platform::Netflix => "Netflix",
            Platform::PrimeVideo => "Prime Video",
            Platform::DisneyPlus => "Disney+",
            Platform::HboMax => "HBO Max",
        }
    }

    /// La plateforme d'un identifiant (`None` pour `all`, vide ou inconnu).
    pub fn from_slug(slug: &str) -> Option<Platform> {
        Platform::ALL.into_iter().find(|p| p.slug() == slug)
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Film ou série.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(crate = "paquets::serde")]
pub enum Kind {
    /// Un long métrage.
    #[serde(rename = "film")]
    Film,
    /// Une série ou une mini-série.
    #[serde(rename = "serie")]
    Serie,
}

impl Kind {
    /// L'identifiant dans les URL et le JSON : `film`, `serie`.
    pub fn slug(self) -> &'static str {
        match self {
            Kind::Film => "film",
            Kind::Serie => "serie",
        }
    }

    /// Le nom affiché.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Film => "Film",
            Kind::Serie => "Série",
        }
    }

    /// Le format d'un identifiant (`None` pour `all`, vide ou inconnu).
    pub fn from_slug(slug: &str) -> Option<Kind> {
        match slug {
            "film" => Some(Kind::Film),
            "serie" => Some(Kind::Serie),
            _ => None,
        }
    }
}

/// Un genre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(crate = "paquets::serde")]
pub enum Genre {
    /// Action.
    #[serde(rename = "action")]
    Action,
    /// Aventure.
    #[serde(rename = "aventure")]
    Aventure,
    /// Animation.
    #[serde(rename = "animation")]
    Animation,
    /// Comédie.
    #[serde(rename = "comedie")]
    Comedie,
    /// Policier.
    #[serde(rename = "policier")]
    Crime,
    /// Documentaire.
    #[serde(rename = "documentaire")]
    Documentaire,
    /// Drame.
    #[serde(rename = "drame")]
    Drame,
    /// Famille.
    #[serde(rename = "famille")]
    Famille,
    /// Fantastique.
    #[serde(rename = "fantastique")]
    Fantastique,
    /// Historique.
    #[serde(rename = "historique")]
    Historique,
    /// Horreur.
    #[serde(rename = "horreur")]
    Horreur,
    /// Mystère.
    #[serde(rename = "mystere")]
    Mystere,
    /// Romance.
    #[serde(rename = "romance")]
    Romance,
    /// Science-fiction.
    #[serde(rename = "sf")]
    Sf,
    /// Thriller.
    #[serde(rename = "thriller")]
    Thriller,
}

impl Genre {
    /// Tous les genres, dans l'ordre d'affichage.
    pub const ALL: [Genre; 15] = [
        Genre::Action,
        Genre::Aventure,
        Genre::Animation,
        Genre::Comedie,
        Genre::Crime,
        Genre::Documentaire,
        Genre::Drame,
        Genre::Famille,
        Genre::Fantastique,
        Genre::Historique,
        Genre::Horreur,
        Genre::Mystere,
        Genre::Romance,
        Genre::Sf,
        Genre::Thriller,
    ];

    /// L'identifiant dans le JSON.
    pub fn slug(self) -> &'static str {
        match self {
            Genre::Action => "action",
            Genre::Aventure => "aventure",
            Genre::Animation => "animation",
            Genre::Comedie => "comedie",
            Genre::Crime => "policier",
            Genre::Documentaire => "documentaire",
            Genre::Drame => "drame",
            Genre::Famille => "famille",
            Genre::Fantastique => "fantastique",
            Genre::Historique => "historique",
            Genre::Horreur => "horreur",
            Genre::Mystere => "mystere",
            Genre::Romance => "romance",
            Genre::Sf => "sf",
            Genre::Thriller => "thriller",
        }
    }

    /// Le nom affiché.
    pub fn label(self) -> &'static str {
        match self {
            Genre::Action => "Action",
            Genre::Aventure => "Aventure",
            Genre::Animation => "Animation",
            Genre::Comedie => "Comédie",
            Genre::Crime => "Policier",
            Genre::Documentaire => "Documentaire",
            Genre::Drame => "Drame",
            Genre::Famille => "Famille",
            Genre::Fantastique => "Fantastique",
            Genre::Historique => "Historique",
            Genre::Horreur => "Horreur",
            Genre::Mystere => "Mystère",
            Genre::Romance => "Romance",
            Genre::Sf => "Science-fiction",
            Genre::Thriller => "Thriller",
        }
    }

    /// L'emoji d'une affiche sans image, selon le genre.
    pub fn emoji(self) -> &'static str {
        match self {
            Genre::Action => "💥",
            Genre::Aventure => "🧭",
            Genre::Animation => "🎨",
            Genre::Comedie => "😂",
            Genre::Crime => "🕵️",
            Genre::Documentaire => "🎥",
            Genre::Drame => "🎭",
            Genre::Famille => "👨‍👩‍👧",
            Genre::Fantastique => "🐉",
            Genre::Historique => "🏛️",
            Genre::Horreur => "👻",
            Genre::Mystere => "🔍",
            Genre::Romance => "💘",
            Genre::Sf => "🚀",
            Genre::Thriller => "🔪",
        }
    }

    /// Le genre d'un identifiant.
    pub fn from_slug(slug: &str) -> Option<Genre> {
        Genre::ALL.into_iter().find(|g| g.slug() == slug)
    }

    /// Les genres de CineShuffle d'un genre de TMDB (films et séries).
    pub fn from_tmdb(id: u32) -> &'static [Genre] {
        match id {
            28 => &[Genre::Action],
            12 | 37 => &[Genre::Aventure],
            10759 => &[Genre::Action, Genre::Aventure],
            16 => &[Genre::Animation],
            35 => &[Genre::Comedie],
            80 => &[Genre::Crime],
            99 => &[Genre::Documentaire],
            18 => &[Genre::Drame],
            10751 | 10762 => &[Genre::Famille],
            14 => &[Genre::Fantastique],
            10765 => &[Genre::Sf, Genre::Fantastique],
            36 | 10752 | 10768 => &[Genre::Historique],
            27 => &[Genre::Horreur],
            9648 => &[Genre::Mystere],
            10749 => &[Genre::Romance],
            878 => &[Genre::Sf],
            53 => &[Genre::Thriller],
            _ => &[],
        }
    }

    /// Le genre TMDB correspondant, pour les films ou les séries (`None` s'il n'existe pas).
    pub fn tmdb_id(self, kind: Kind) -> Option<u32> {
        match (self, kind) {
            (Genre::Action, Kind::Film) => Some(28),
            (Genre::Aventure, Kind::Film) => Some(12),
            (Genre::Action | Genre::Aventure, Kind::Serie) => Some(10759),
            (Genre::Animation, _) => Some(16),
            (Genre::Comedie, _) => Some(35),
            (Genre::Crime, _) => Some(80),
            (Genre::Documentaire, _) => Some(99),
            (Genre::Drame, _) => Some(18),
            (Genre::Famille, _) => Some(10751),
            (Genre::Fantastique, Kind::Film) => Some(14),
            (Genre::Fantastique | Genre::Sf, Kind::Serie) => Some(10765),
            (Genre::Historique, Kind::Film) => Some(36),
            (Genre::Historique, Kind::Serie) => Some(10768),
            (Genre::Horreur, Kind::Film) => Some(27),
            (Genre::Mystere, _) => Some(9648),
            (Genre::Romance, Kind::Film) => Some(10749),
            (Genre::Sf, Kind::Film) => Some(878),
            (Genre::Thriller, Kind::Film) => Some(53),
            _ => None,
        }
    }

    fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

/// Le masque d'une liste de genres.
fn mask(genres: &[Genre]) -> u16 {
    genres.iter().fold(0, |m, g| m | g.bit())
}

/// Un film ou une série.
#[data]
#[derive(PartialEq)]
pub struct Title {
    /// L'identifiant stable : `movie:603` et `tv:1399` (TMDB) ou `local:12` (sélection embarquée).
    pub key: String,
    /// Le titre, en français quand il existe.
    pub name: String,
    /// L'année de sortie.
    #[serde(default)]
    pub year: Option<u16>,
    /// Film ou série.
    pub kind: Kind,
    /// Les plateformes qui le proposent en France (parmi les quatre de CineShuffle).
    #[serde(default)]
    pub platforms: Vec<Platform>,
    /// Ses genres.
    #[serde(default)]
    pub genres: Vec<Genre>,
    /// Le résumé.
    #[serde(default)]
    pub overview: String,
    /// L'adresse de l'affiche (TMDB), s'il y en a une.
    #[serde(default)]
    pub poster: Option<String>,
    /// Les acteurs principaux.
    #[serde(default)]
    pub cast: Vec<String>,
    /// La durée d'un film, ou le nombre de saisons d'une série.
    #[serde(default)]
    pub length: Option<String>,
    /// La note moyenne sur 10 (TMDB).
    #[serde(default)]
    pub rating: Option<f32>,
    /// L'emoji de l'affiche dessinée (quand il n'y a pas d'image).
    pub emoji: String,
    /// La teinte de l'affiche dessinée (0 à 359).
    pub hue: u16,
}

impl Title {
    /// Le masque des genres.
    pub fn genre_mask(&self) -> u16 {
        mask(&self.genres)
    }

    /// Les genres, séparés par des virgules.
    pub fn genre_labels(&self) -> String {
        self.genres
            .iter()
            .map(|g| g.label())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Les plateformes, séparées par des virgules.
    pub fn platform_labels(&self) -> String {
        self.platforms
            .iter()
            .map(|p| p.label())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// `2019 · Film · 2 h 13`.
    pub fn meta(&self) -> String {
        let mut parts = Vec::new();
        if let Some(year) = self.year {
            parts.push(year.to_string());
        }
        parts.push(self.kind.label().to_owned());
        if let Some(length) = &self.length {
            parts.push(length.clone());
        }
        parts.join(" · ")
    }

    /// `Avec A, B et C`, ou rien sans distribution connue.
    pub fn cast_line(&self) -> Option<String> {
        match self.cast.as_slice() {
            [] => None,
            [one] => Some(format!("Avec {one}")),
            [rest @ .., last] => Some(format!("Avec {} et {last}", rest.join(", "))),
        }
    }

    /// Ce qu'il faut retenir d'un titre voté.
    pub fn seen(&self) -> Seen {
        Seen {
            key: self.key.clone(),
            name: self.name.clone(),
            kind: self.kind,
            genres: self.genres.clone(),
            platforms: self.platforms.clone(),
        }
    }
}

/// Une teinte stable pour une clé (les affiches dessinées des titres TMDB sans image).
pub fn hue_for(key: &str) -> u16 {
    let hash = key.bytes().fold(0x811c_9dc5_u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    });
    (hash % 360) as u16
}

/// Ce que le moteur garde d'un titre voté : de quoi comparer les autres titres.
#[data]
#[derive(PartialEq)]
pub struct Seen {
    /// La clé du titre.
    pub key: String,
    /// Son nom.
    pub name: String,
    /// Film ou série.
    pub kind: Kind,
    /// Ses genres.
    #[serde(default)]
    pub genres: Vec<Genre>,
    /// Ses plateformes.
    #[serde(default)]
    pub platforms: Vec<Platform>,
}

macro_rules! titles {
    ($( $id:literal, $name:literal, $year:literal, $kind:ident, $platform:ident, [$($genre:ident),+], $length:literal, $emoji:literal, $hue:literal, $pitch:literal; )+) => {
        /// La sélection embarquée (secours sans clé TMDB, et hors ligne).
        fn builtin_titles() -> Vec<Title> {
            vec![
                $( Title {
                    key: concat!("local:", $id).to_owned(),
                    name: $name.to_owned(),
                    year: Some($year),
                    kind: Kind::$kind,
                    platforms: vec![Platform::$platform],
                    genres: vec![$(Genre::$genre),+],
                    overview: $pitch.to_owned(),
                    poster: None,
                    cast: Vec::new(),
                    length: Some($length.to_owned()),
                    rating: None,
                    emoji: $emoji.to_owned(),
                    hue: $hue,
                }, )+
            ]
        }
    };
}

titles! {
    // Netflix
    1, "Stranger Things", 2016, Serie, Netflix, [Sf, Horreur, Aventure], "5 saisons", "👾", 350,
        "Une bande d'ados d'Hawkins affronte des créatures venues d'un monde à l'envers.";
    2, "Mercredi", 2022, Serie, Netflix, [Comedie, Mystere, Fantastique], "En cours", "🖤", 270,
        "Mercredi Addams mène l'enquête à Nevermore avec un humour plus noir que ses tresses.";
    3, "Squid Game", 2021, Serie, Netflix, [Thriller, Drame], "3 saisons", "🦑", 330,
        "456 joueurs endettés, des jeux d'enfants mortels et un prix colossal.";
    4, "The Crown", 2016, Serie, Netflix, [Drame, Historique], "6 saisons", "👑", 45,
        "Le règne d'Élisabeth II, des coulisses du palais aux crises de la Couronne.";
    5, "Lupin", 2021, Serie, Netflix, [Crime, Mystere], "En cours", "🎩", 210,
        "Assane Diop venge son père avec l'audace du gentleman cambrioleur.";
    6, "Dark", 2017, Serie, Netflix, [Sf, Mystere, Drame], "3 saisons", "⏳", 190,
        "Des disparitions à Winden révèlent un nœud temporel sur quatre générations.";
    7, "La casa de papel", 2017, Serie, Netflix, [Crime, Thriller, Action], "5 parties", "🎭", 0,
        "Le Professeur et sa bande prennent d'assaut la Fabrique nationale de la monnaie.";
    8, "Le Jeu de la dame", 2020, Serie, Netflix, [Drame], "7 épisodes", "♟️", 30,
        "Une orpheline prodige des échecs grimpe jusqu'au sommet, entre génie et addictions.";
    9, "Arcane", 2021, Serie, Netflix, [Animation, Fantastique, Action], "2 saisons", "⚙️", 290,
        "Deux sœurs séparées entre Piltover et Zaun, dans une animation éblouissante.";
    10, "La Chronique des Bridgerton", 2020, Serie, Netflix, [Romance, Historique, Drame], "En cours", "💌", 320,
        "Bals, rumeurs et mariages arrangés dans la haute société de la Régence.";
    11, "Narcos", 2015, Serie, Netflix, [Crime, Drame, Historique], "3 saisons", "💼", 80,
        "La traque de Pablo Escobar et des cartels colombiens par la DEA.";
    12, "BoJack Horseman", 2014, Serie, Netflix, [Animation, Comedie, Drame], "6 saisons", "🐴", 20,
        "Une star déchue de sitcom (et cheval) cherche un sens à sa vie à Hollywoo.";
    13, "Mindhunter", 2017, Serie, Netflix, [Crime, Thriller], "2 saisons", "🕵️", 160,
        "Deux agents du FBI inventent le profilage en interrogeant des tueurs en série.";
    14, "The Irishman", 2019, Film, Netflix, [Crime, Drame, Historique], "3 h 29", "🥃", 25,
        "Scorsese retrace cinquante ans de mafia à travers un tueur à gages vieillissant.";
    15, "Glass Onion", 2022, Film, Netflix, [Mystere, Comedie, Crime], "2 h 19", "🧅", 55,
        "Benoit Blanc démêle un meurtre sur l'île privée d'un milliardaire de la tech.";
    16, "Roma", 2018, Film, Netflix, [Drame], "2 h 15", "🎞️", 220,
        "Le quotidien d'une domestique à Mexico au début des années 1970, en noir et blanc.";
    17, "Don't Look Up", 2021, Film, Netflix, [Comedie, Sf], "2 h 18", "☄️", 15,
        "Deux astronomes alertent le monde d'une comète tueuse : personne ne veut l'entendre.";
    18, "Le Problème à 3 corps", 2024, Serie, Netflix, [Sf, Mystere], "En cours", "🌌", 240,
        "Des scientifiques découvrent qu'une civilisation lointaine a les yeux sur la Terre.";
    19, "Adolescence", 2025, Serie, Netflix, [Drame, Crime], "4 épisodes", "🎥", 200,
        "Un garçon de 13 ans est accusé de meurtre : quatre épisodes en plans-séquences.";
    20, "KPop Demon Hunters", 2025, Film, Netflix, [Animation, Fantastique, Action], "1 h 40", "🎤", 300,
        "Un groupe de K-pop chasse les démons entre deux concerts à guichets fermés.";

    // HBO Max
    21, "Succession", 2018, Serie, HboMax, [Drame, Comedie], "4 saisons", "📈", 40,
        "Les enfants Roy se déchirent pour l'héritage d'un empire médiatique.";
    22, "The Last of Us", 2023, Serie, HboMax, [Drame, Horreur, Aventure], "En cours", "🍄", 100,
        "Joel escorte Ellie à travers une Amérique ravagée par un champignon.";
    23, "Game of Thrones", 2011, Serie, HboMax, [Fantastique, Drame, Action], "8 saisons", "🐉", 10,
        "Les grandes maisons de Westeros se disputent le Trône de fer, l'hiver arrive.";
    24, "House of the Dragon", 2022, Serie, HboMax, [Fantastique, Drame], "En cours", "🔥", 5,
        "Deux cents ans avant Game of Thrones, les Targaryen s'entre-déchirent.";
    25, "Sur écoute (The Wire)", 2002, Serie, HboMax, [Crime, Drame], "5 saisons", "🚨", 215,
        "Baltimore vue par ses flics, ses dealers, ses dockers, ses élus et ses écoles.";
    26, "Les Soprano", 1999, Serie, HboMax, [Crime, Drame], "6 saisons", "🍝", 355,
        "Tony Soprano, parrain du New Jersey, consulte une psy pour ses crises d'angoisse.";
    27, "Chernobyl", 2019, Serie, HboMax, [Drame, Historique], "5 épisodes", "☢️", 75,
        "La catastrophe de 1986 et ceux qui ont tout sacrifié pour la contenir.";
    28, "The White Lotus", 2021, Serie, HboMax, [Comedie, Drame, Mystere], "En cours", "🌺", 150,
        "Une semaine dans un palace de luxe, des clients odieux et un cadavre.";
    29, "Euphoria", 2019, Serie, HboMax, [Drame], "En cours", "✨", 280,
        "Rue et ses amis traversent drogues, amour et identité au lycée.";
    30, "True Detective", 2014, Serie, HboMax, [Crime, Mystere, Thriller], "4 saisons", "🔦", 60,
        "Des enquêteurs hantés traquent des meurtres rituels, une saison par affaire.";
    31, "Barry", 2018, Serie, HboMax, [Comedie, Crime], "4 saisons", "🎬", 35,
        "Un tueur à gages se découvre une passion pour le théâtre à Los Angeles.";
    32, "Mare of Easttown", 2021, Serie, HboMax, [Crime, Drame, Mystere], "7 épisodes", "🧩", 205,
        "Une inspectrice de Pennsylvanie enquête sur un meurtre en pleine crise familiale.";
    33, "The Penguin", 2024, Serie, HboMax, [Crime, Drame], "8 épisodes", "🐧", 230,
        "Oz Cobb s'empare de la pègre de Gotham après la chute des Falcone.";
    34, "Frères d'armes", 2001, Serie, HboMax, [Historique, Action, Drame], "10 épisodes", "🎖️", 90,
        "La Easy Company, du débarquement en Normandie jusqu'au nid d'aigle d'Hitler.";
    35, "The Pitt", 2025, Serie, HboMax, [Drame], "En cours", "🏥", 180,
        "Quinze heures de garde aux urgences de Pittsburgh, une heure par épisode.";
    36, "Dune : Deuxième partie", 2024, Film, HboMax, [Sf, Aventure], "2 h 46", "🏜️", 38,
        "Paul Atréides rejoint les Fremen d'Arrakis et marche vers son destin.";
    37, "The Batman", 2022, Film, HboMax, [Action, Crime], "2 h 56", "🦇", 0,
        "Un Batman détective traque l'Homme-Mystère dans un Gotham poisseux.";
    38, "Barbie", 2023, Film, HboMax, [Comedie, Fantastique], "1 h 54", "🎀", 325,
        "Barbie quitte Barbieland pour le monde réel et ses questions existentielles.";
    39, "Mad Max: Fury Road", 2015, Film, HboMax, [Action, Sf], "2 h 00", "🛞", 28,
        "Une course-poursuite furieuse à travers un désert post-apocalyptique.";
    40, "Hacks", 2021, Serie, HboMax, [Comedie], "En cours", "🎙️", 50,
        "Une légende du stand-up de Las Vegas et une jeune autrice que tout oppose.";

    // Disney+
    41, "The Mandalorian", 2019, Serie, DisneyPlus, [Sf, Aventure, Action], "3 saisons", "🪖", 35,
        "Un chasseur de primes solitaire protège un mystérieux enfant de la Force.";
    42, "Andor", 2022, Serie, DisneyPlus, [Sf, Thriller, Drame], "2 saisons", "🛰️", 195,
        "Cassian Andor, petit voleur, devient l'un des premiers rebelles contre l'Empire.";
    43, "Loki", 2021, Serie, DisneyPlus, [Fantastique, Sf, Aventure], "2 saisons", "🌀", 110,
        "Le dieu de la malice, arrêté par le TVA, répare le temps qu'il a brisé.";
    44, "The Bear", 2022, Serie, DisneyPlus, [Comedie, Drame], "En cours", "🍳", 20,
        "Un chef étoilé reprend la sandwicherie familiale de Chicago, dans le chaos.";
    45, "Shōgun", 2024, Serie, DisneyPlus, [Historique, Drame], "En cours", "⚔️", 355,
        "Au Japon de 1600, un navigateur anglais se retrouve au cœur d'une guerre de clans.";
    46, "Only Murders in the Building", 2021, Serie, DisneyPlus, [Comedie, Mystere, Crime], "En cours", "🏙️", 25,
        "Trois voisins fans de true crime lancent leur podcast sur un meurtre dans leur immeuble.";
    47, "WandaVision", 2021, Serie, DisneyPlus, [Fantastique, Comedie, Mystere], "9 épisodes", "📺", 340,
        "Wanda et Vision vivent une sitcom parfaite… trop parfaite pour être vraie.";
    48, "Percy Jackson et les Olympiens", 2023, Serie, DisneyPlus, [Fantastique, Aventure, Famille], "En cours", "🔱", 185,
        "Un ado découvre qu'il est le fils de Poséidon et part en quête à travers l'Amérique.";
    49, "Coco", 2017, Film, DisneyPlus, [Animation, Famille, Fantastique], "1 h 45", "💀", 15,
        "Miguel rêve de musique et se retrouve au Pays des morts pendant la fête des morts.";
    50, "Vice-versa 2", 2024, Film, DisneyPlus, [Animation, Famille, Comedie], "1 h 36", "😊", 265,
        "Riley a 13 ans et de nouvelles émotions, dont Anxiété, s'installent au QG.";
    51, "Soul", 2020, Film, DisneyPlus, [Animation, Famille, Drame], "1 h 40", "🎷", 235,
        "Un pianiste de jazz se retrouve séparé de son corps à la veille de son grand soir.";
    52, "Avengers : Endgame", 2019, Film, DisneyPlus, [Action, Sf, Aventure], "3 h 01", "🛡️", 265,
        "Les Avengers survivants tentent l'impossible pour réparer le claquement de doigts.";
    53, "Black Panther", 2018, Film, DisneyPlus, [Action, Sf, Aventure], "2 h 14", "🐆", 275,
        "T'Challa devient roi du Wakanda et doit défendre son trône et son peuple.";
    54, "Zootopie", 2016, Film, DisneyPlus, [Animation, Famille, Comedie, Mystere], "1 h 48", "🦊", 30,
        "Une lapine policière et un renard arnaqueur enquêtent dans une ville d'animaux.";
    55, "Le Roi lion", 1994, Film, DisneyPlus, [Animation, Famille, Drame], "1 h 28", "🦁", 42,
        "Simba, héritier de la terre des lions, doit reprendre la place qui lui revient.";
    56, "Avatar : La Voie de l'eau", 2022, Film, DisneyPlus, [Sf, Aventure, Action], "3 h 12", "🌊", 195,
        "Jake Sully et sa famille trouvent refuge auprès des clans de la mer de Pandora.";
    57, "Les Gardiens de la Galaxie Vol. 3", 2023, Film, DisneyPlus, [Sf, Comedie, Action], "2 h 30", "🦝", 300,
        "Les Gardiens partent sauver Rocket et affrontent le passé de leur ami.";
    58, "Rogue One", 2016, Film, DisneyPlus, [Sf, Action], "2 h 13", "⭐", 210,
        "Une poignée de rebelles vole les plans de l'Étoile de la mort.";
    59, "Ratatouille", 2007, Film, DisneyPlus, [Animation, Famille, Comedie], "1 h 51", "🧀", 48,
        "Un rat qui rêve d'être chef cuisine en secret dans un grand restaurant parisien.";
    60, "WALL-E", 2008, Film, DisneyPlus, [Animation, Sf, Famille], "1 h 38", "🤖", 32,
        "Un petit robot nettoyeur, seul sur Terre, tombe amoureux d'une sonde venue de l'espace.";

    // Prime Video
    61, "The Boys", 2019, Serie, PrimeVideo, [Action, Comedie, Sf], "En cours", "🦸", 0,
        "Une bande de justiciers sans pouvoirs s'attaque à des super-héros corrompus et adulés.";
    62, "Fallout", 2024, Serie, PrimeVideo, [Sf, Aventure, Action], "En cours", "☢️", 95,
        "Deux siècles après l'apocalypse nucléaire, Lucy quitte son abri pour la surface.";
    63, "Reacher", 2022, Serie, PrimeVideo, [Action, Crime, Thriller], "En cours", "💪", 215,
        "Jack Reacher, ancien enquêteur militaire, débarque en ville et règle les problèmes à sa façon.";
    64, "Invincible", 2021, Serie, PrimeVideo, [Animation, Action, Sf], "En cours", "🦺", 50,
        "Le fils du plus grand super-héros de la Terre découvre ses pouvoirs… et les secrets de son père.";
    65, "La Fabuleuse Mme Maisel", 2017, Serie, PrimeVideo, [Comedie, Drame, Historique], "5 saisons", "🎙️", 330,
        "Dans le New York des années 1950, une femme au foyer se lance dans le stand-up.";
    66, "Good Omens", 2019, Serie, PrimeVideo, [Fantastique, Comedie], "En cours", "😇", 280,
        "Un ange et un démon, amis de longue date, tentent d'empêcher la fin du monde.";
    67, "Les Anneaux de pouvoir", 2022, Serie, PrimeVideo, [Fantastique, Aventure, Drame], "En cours", "💍", 40,
        "Des milliers d'années avant Le Hobbit, la forge des anneaux et le retour du mal en Terre du Milieu.";
    68, "Jack Ryan", 2018, Serie, PrimeVideo, [Action, Thriller], "4 saisons", "🕴️", 205,
        "Un analyste de la CIA est propulsé sur le terrain face à des menaces mondiales.";
    69, "Mr. & Mrs. Smith", 2024, Serie, PrimeVideo, [Action, Comedie, Romance], "En cours", "💼", 350,
        "Deux espions inconnus l'un de l'autre sont mariés pour les besoins de leurs missions.";
    70, "Gen V", 2023, Serie, PrimeVideo, [Action, Comedie, Sf], "En cours", "🎓", 310,
        "Dans une université pour jeunes super-héros, les élèves découvrent un secret inquiétant.";
    71, "La Roue du temps", 2021, Serie, PrimeVideo, [Fantastique, Aventure], "3 saisons", "🌀", 255,
        "Une magicienne guide cinq jeunes gens dont l'un pourrait être le Dragon réincarné.";
    72, "Upload", 2020, Serie, PrimeVideo, [Comedie, Sf, Romance], "4 saisons", "☁️", 190,
        "Après sa mort, un jeune homme est téléversé dans un paradis numérique très commercial.";
    73, "Fleabag", 2016, Serie, PrimeVideo, [Comedie, Drame], "2 saisons", "🦊", 5,
        "Une Londonienne caustique et brisée raconte sa vie en regardant la caméra.";
    74, "Citadel", 2023, Serie, PrimeVideo, [Action, Thriller, Sf], "En cours", "🛡️", 220,
        "Deux anciens agents d'une agence d'espionnage disparue reprennent du service, sans mémoire.";
    75, "Saltburn", 2023, Film, PrimeVideo, [Thriller, Drame, Comedie], "2 h 11", "🏰", 140,
        "Un étudiant d'Oxford est invité à passer l'été dans le domaine extravagant d'un camarade.";
    76, "Road House", 2024, Film, PrimeVideo, [Action, Thriller], "2 h 01", "🥊", 25,
        "Un ancien combattant de l'UFC devient videur dans un bar mal famé des Keys de Floride.";
    77, "Sound of Metal", 2019, Film, PrimeVideo, [Drame], "2 h 00", "🥁", 200,
        "Un batteur de métal perd l'ouïe et doit réapprendre à vivre.";
    78, "Air", 2023, Film, PrimeVideo, [Drame, Historique], "1 h 52", "👟", 30,
        "Comment Nike a convaincu un jeune Michael Jordan de signer et créé une légende.";
    79, "The Tomorrow War", 2021, Film, PrimeVideo, [Sf, Action], "2 h 18", "👽", 170,
        "Des soldats du futur recrutent des civils d'aujourd'hui pour une guerre contre des extraterrestres.";
    80, "Being the Ricardos", 2021, Film, PrimeVideo, [Drame, Historique, Comedie], "2 h 05", "📺", 45,
        "Une semaine décisive dans la vie de Lucille Ball et Desi Arnaz, stars de la télévision.";
}

/// La sélection embarquée : 80 titres, 20 par plateforme.
pub fn builtin() -> &'static [Title] {
    static BUILTIN: OnceLock<Vec<Title>> = OnceLock::new();
    BUILTIN.get_or_init(builtin_titles)
}

/// Un titre de la sélection embarquée, par sa clé.
pub fn builtin_title(key: &str) -> Option<&'static Title> {
    builtin().iter().find(|t| t.key == key)
}

/// Un générateur pseudo-aléatoire (splitmix64) : rapide, sans dépendance, et identique
/// sur le serveur, dans le navigateur et dans l'application iOS pour une même graine.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// Un générateur à partir d'une graine.
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    /// Le nombre suivant.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Un entier dans `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// La graine de la sélection du jour : le numéro du jour depuis 1970.
pub fn day_seed(unix_seconds: u64) -> u64 {
    unix_seconds / 86_400
}

/// Le nombre de titres d'un tirage.
pub const PICKS: usize = 5;

/// Tire `n` titres de `pool` au hasard, sans doublon, parmi ceux de `platform` et
/// `kind` (`None` : tous). La même graine donne toujours le même tirage.
pub fn picks(
    pool: &[Title],
    seed: u64,
    platform: Option<Platform>,
    kind: Option<Kind>,
    n: usize,
) -> Vec<Title> {
    let mut found: Vec<&Title> = pool
        .iter()
        .filter(|t| platform.is_none_or(|p| t.platforms.contains(&p)))
        .filter(|t| kind.is_none_or(|k| t.kind == k))
        .collect();
    let mut rng = Rng::new(seed ^ 0x005E_ED0F_F11E);
    let n = n.min(found.len());
    // Fisher-Yates partiel : seuls les `n` premiers sont mélangés.
    for i in 0..n {
        let j = i + rng.below(found.len() - i);
        found.swap(i, j);
    }
    found.into_iter().take(n).cloned().collect()
}

/// La similarité de deux titres (0 à 1) : genres en commun (indice de Jaccard), un
/// peu plus pour le même format et une plateforme commune.
pub fn similarity(a: &Title, b: &Seen) -> f32 {
    let (ma, mb) = (a.genre_mask(), mask(&b.genres));
    let union = (ma | mb).count_ones();
    let jaccard = if union == 0 {
        0.0
    } else {
        (ma & mb).count_ones() as f32 / union as f32
    };
    let mut score = jaccard * 0.8;
    if a.kind == b.kind {
        score += 0.12;
    }
    if a.platforms.iter().any(|p| b.platforms.contains(p)) {
        score += 0.08;
    }
    score
}

/// Une suggestion.
#[data]
#[derive(PartialEq)]
pub struct Suggestion {
    /// Le titre suggéré.
    pub title: Title,
    /// Son score (plus il est haut, mieux c'est).
    pub score: f32,
    /// Pourquoi il est suggéré.
    pub reason: String,
}

/// Les goûts d'une personne, tels que le moteur les lit.
#[data]
#[derive(Default, PartialEq)]
pub struct Profile {
    /// Les titres aimés.
    #[serde(default)]
    pub liked: Vec<Seen>,
    /// Les titres vus mais pas aimés.
    #[serde(default)]
    pub disliked: Vec<Seen>,
    /// Ses plateformes (vide : toutes).
    #[serde(default)]
    pub platforms: Vec<Platform>,
    /// Ses genres préférés.
    #[serde(default)]
    pub genres: Vec<Genre>,
    /// Films, séries ou les deux (`None`).
    #[serde(default)]
    pub kind: Option<Kind>,
}

impl Profile {
    /// Aucun vote ni préférence ?
    pub fn is_empty(&self) -> bool {
        *self == Profile::default()
    }

    /// Les genres les plus présents dans les goûts (préférés, puis aimés), au plus `n`.
    pub fn top_genres(&self, n: usize) -> Vec<Genre> {
        let mut weights: Vec<(Genre, f32)> = Genre::ALL.iter().map(|&g| (g, 0.0)).collect();
        for (genre, weight) in &mut weights {
            if self.genres.contains(genre) {
                *weight += 1.5;
            }
            *weight += self
                .liked
                .iter()
                .filter(|s| s.genres.contains(genre))
                .count() as f32;
            *weight -= 0.5
                * self
                    .disliked
                    .iter()
                    .filter(|s| s.genres.contains(genre))
                    .count() as f32;
        }
        weights.retain(|(_, w)| *w > 0.0);
        weights.sort_by(|a, b| b.1.total_cmp(&a.1));
        weights.into_iter().take(n).map(|(g, _)| g).collect()
    }

    fn voted(&self, key: &str) -> bool {
        self.liked
            .iter()
            .chain(&self.disliked)
            .any(|s| s.key == key)
    }
}

/// Les `n` meilleures suggestions pour `profile` parmi `candidates` (les plus
/// populaires d'abord : leur rang donne un petit bonus), sans les titres déjà votés.
/// `popular` est la raison donnée quand rien d'autre ne l'explique.
pub fn recommend(
    candidates: &[Title],
    profile: &Profile,
    n: usize,
    popular: &str,
) -> Vec<Suggestion> {
    let preferred = mask(&profile.genres);
    let mut seen_keys = Vec::new();
    let mut found: Vec<Suggestion> = candidates
        .iter()
        .filter(|t| !profile.voted(&t.key))
        .filter(|t| {
            profile.platforms.is_empty()
                || t.platforms.is_empty()
                || t.platforms.iter().any(|p| profile.platforms.contains(p))
        })
        .filter(|t| profile.kind.is_none_or(|k| t.kind == k))
        .filter(|t| {
            // Un même titre peut venir de plusieurs listes.
            let new = !seen_keys.contains(&t.key);
            seen_keys.push(t.key.clone());
            new
        })
        .enumerate()
        .map(|(rank, t)| {
            let best = profile
                .liked
                .iter()
                .map(|l| (similarity(t, l), l))
                .max_by(|a, b| a.0.total_cmp(&b.0));
            let from_likes: f32 = profile.liked.iter().map(|l| similarity(t, l)).sum();
            let from_dislikes: f32 = profile.disliked.iter().map(|d| similarity(t, d)).sum();
            let shared = t.genre_mask() & preferred;
            let from_genres = shared.count_ones() as f32 * 0.6;
            let popularity = (0.3 - rank as f32 * 0.02).max(0.0);
            let score = from_likes * 1.5 - from_dislikes + from_genres + popularity;
            let reason = match best {
                Some((s, l)) if s >= 0.35 => format!("Parce que vous avez aimé « {} »", l.name),
                _ if shared != 0 => format!(
                    "Dans vos genres : {}",
                    t.genres
                        .iter()
                        .filter(|g| shared & g.bit() != 0)
                        .map(|g| g.label())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                _ => popular.to_owned(),
            };
            Suggestion {
                title: t.clone(),
                score,
                reason,
            }
        })
        .collect();
    found.sort_by(|a, b| b.score.total_cmp(&a.score));
    found.truncate(n);
    found
}

/// Les valeurs sûres de la sélection embarquée, en tête des candidats.
const CLASSICS: [u32; 14] = [27, 8, 44, 6, 42, 21, 61, 49, 9, 25, 62, 15, 60, 32];

/// La sélection embarquée, les valeurs sûres d'abord (les candidats de `recommend`).
pub fn builtin_candidates() -> Vec<Title> {
    let mut all: Vec<Title> = builtin().to_vec();
    all.sort_by_key(|t| {
        let id: u32 = t.key.trim_start_matches("local:").parse().unwrap_or(0);
        CLASSICS
            .iter()
            .position(|&c| c == id)
            .unwrap_or(CLASSICS.len() + id as usize)
    });
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_is_consistent() {
        let all = builtin();
        assert_eq!(all.len(), 80);
        for (i, t) in all.iter().enumerate() {
            assert_eq!(t.key, format!("local:{}", i + 1), "ids follow each other");
            assert!(!t.genres.is_empty() && t.hue < 360 && !t.overview.is_empty());
        }
        for p in Platform::ALL {
            assert_eq!(all.iter().filter(|t| t.platforms.contains(&p)).count(), 20);
            assert_eq!(Platform::from_slug(p.slug()), Some(p));
        }
        for g in Genre::ALL {
            assert_eq!(Genre::from_slug(g.slug()), Some(g));
            let json = paquets::serde_json::to_string(&g).unwrap();
            assert_eq!(json, format!("\"{}\"", g.slug()), "serde follows the slugs");
        }
        let json = paquets::serde_json::to_string(&Platform::PrimeVideo).unwrap();
        assert_eq!(json, "\"prime-video\"");
    }

    #[test]
    fn picks_are_random_but_reproducible() {
        let a = picks(builtin(), 42, None, None, PICKS);
        assert_eq!(a.len(), 5);
        // La même valeur est vérifiée par les tests de l'application iOS.
        let keys: Vec<&str> = a.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(
            keys,
            ["local:68", "local:39", "local:4", "local:10", "local:25"]
        );
        assert_eq!(a, picks(builtin(), 42, None, None, PICKS));
        assert_ne!(a, picks(builtin(), 43, None, None, PICKS));
        let prime = picks(
            builtin(),
            7,
            Some(Platform::PrimeVideo),
            Some(Kind::Film),
            PICKS,
        );
        assert!(
            prime
                .iter()
                .all(|t| t.platforms == [Platform::PrimeVideo] && t.kind == Kind::Film)
        );
    }

    #[test]
    fn suggestions_follow_the_votes() {
        let candidates = builtin_candidates();
        let none = recommend(&candidates, &Profile::default(), 4, "Une valeur sûre");
        assert_eq!(none[0].title.key, "local:27");
        assert_eq!(none[0].reason, "Une valeur sûre");

        // Aimer des séries policières fait remonter des séries policières.
        let liked = |key: &str| builtin_title(key).unwrap().seen();
        let profile = Profile {
            liked: vec![liked("local:25"), liked("local:26")],
            ..Profile::default()
        };
        let found = recommend(&candidates, &profile, 3, "");
        assert!(found.iter().all(|s| s.title.genres.contains(&Genre::Crime)));
        assert!(found[0].reason.starts_with("Parce que vous avez aimé"));
        assert!(found.iter().all(|s| s.title.key != "local:25"));
        assert_eq!(profile.top_genres(2), [Genre::Crime, Genre::Drame]);

        let filtered = recommend(
            &candidates,
            &Profile {
                platforms: vec![Platform::Netflix],
                kind: Some(Kind::Film),
                genres: vec![Genre::Animation],
                ..Profile::default()
            },
            2,
            "",
        );
        assert_eq!(filtered[0].title.key, "local:20");
        assert!(filtered[0].reason.starts_with("Dans vos genres"));
    }

    #[test]
    fn tmdb_genres_round_trip() {
        for g in Genre::ALL {
            if let Some(id) = g.tmdb_id(Kind::Film) {
                assert!(Genre::from_tmdb(id).contains(&g), "{g:?}");
            }
        }
        assert_eq!(Genre::from_tmdb(10765), [Genre::Sf, Genre::Fantastique]);
        assert!(Genre::from_tmdb(10770).is_empty());
    }

    #[test]
    fn cast_line() {
        let mut t = builtin()[0].clone();
        assert_eq!(t.cast_line(), None);
        t.cast = vec!["A".into(), "B".into(), "C".into()];
        assert_eq!(t.cast_line().unwrap(), "Avec A, B et C");
    }
}
