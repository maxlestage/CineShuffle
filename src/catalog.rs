//! Le catalogue, le tirage au sort et le moteur de suggestions.
//!
//! Ce module est compilé deux fois : dans le serveur (pages, API JSON) et dans le
//! paquet WebAssembly des îlots. Le même tirage (`picks`) avec la même graine donne
//! donc exactement les mêmes titres des deux côtés.

use std::fmt;

/// Une plateforme de streaming.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    /// Netflix.
    Netflix,
    /// HBO Max.
    HboMax,
    /// Disney+.
    DisneyPlus,
}

impl Platform {
    /// Toutes les plateformes, dans l'ordre d'affichage.
    pub const ALL: [Platform; 3] = [Platform::Netflix, Platform::HboMax, Platform::DisneyPlus];

    /// L'identifiant dans les URL et le JSON : `netflix`, `hbo-max`, `disney-plus`.
    pub fn slug(self) -> &'static str {
        match self {
            Platform::Netflix => "netflix",
            Platform::HboMax => "hbo-max",
            Platform::DisneyPlus => "disney-plus",
        }
    }

    /// Le nom affiché.
    pub fn label(self) -> &'static str {
        match self {
            Platform::Netflix => "Netflix",
            Platform::HboMax => "HBO Max",
            Platform::DisneyPlus => "Disney+",
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Un long métrage.
    Film,
    /// Une série ou une mini-série.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Genre {
    /// Action.
    Action,
    /// Aventure.
    Aventure,
    /// Animation.
    Animation,
    /// Comédie.
    Comedie,
    /// Policier.
    Crime,
    /// Drame.
    Drame,
    /// Famille.
    Famille,
    /// Fantastique.
    Fantastique,
    /// Historique.
    Historique,
    /// Horreur.
    Horreur,
    /// Mystère.
    Mystere,
    /// Romance.
    Romance,
    /// Science-fiction.
    Sf,
    /// Thriller.
    Thriller,
}

impl Genre {
    /// Tous les genres, dans l'ordre d'affichage.
    pub const ALL: [Genre; 14] = [
        Genre::Action,
        Genre::Aventure,
        Genre::Animation,
        Genre::Comedie,
        Genre::Crime,
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

    /// Le genre d'un identifiant.
    pub fn from_slug(slug: &str) -> Option<Genre> {
        Genre::ALL.into_iter().find(|g| g.slug() == slug)
    }

    fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

/// Un film ou une série du catalogue.
#[derive(Debug, PartialEq, Eq)]
pub struct Title {
    /// L'identifiant, stable (votes, API, application iOS).
    pub id: u32,
    /// Le titre français.
    pub name: &'static str,
    /// L'année de sortie.
    pub year: u16,
    /// Film ou série.
    pub kind: Kind,
    /// La plateforme qui le propose (en France, à titre indicatif).
    pub platform: Platform,
    /// Ses genres.
    pub genres: &'static [Genre],
    /// La durée d'un film, ou le nombre de saisons d'une série.
    pub length: &'static str,
    /// Le pitch, en une phrase.
    pub pitch: &'static str,
    /// L'emoji de l'affiche.
    pub emoji: &'static str,
    /// La teinte de l'affiche (0 à 359).
    pub hue: u16,
}

impl Title {
    /// Le masque des genres.
    pub fn genre_mask(&self) -> u16 {
        self.genres.iter().fold(0, |m, g| m | g.bit())
    }

    /// Les genres, séparés par des virgules.
    pub fn genre_labels(&self) -> String {
        self.genres
            .iter()
            .map(|g| g.label())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// `2019 · Film · 2 h 13`.
    pub fn meta(&self) -> String {
        format!("{} · {} · {}", self.year, self.kind.label(), self.length)
    }
}

macro_rules! titles {
    ($( $id:literal, $name:literal, $year:literal, $kind:ident, $platform:ident, [$($genre:ident),+], $length:literal, $emoji:literal, $hue:literal, $pitch:literal; )+) => {
        /// Les 60 titres du catalogue.
        pub static CATALOG: &[Title] = &[
            $( Title {
                id: $id,
                name: $name,
                year: $year,
                kind: Kind::$kind,
                platform: Platform::$platform,
                genres: &[$(Genre::$genre),+],
                length: $length,
                pitch: $pitch,
                emoji: $emoji,
                hue: $hue,
            }, )+
        ];
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
}

/// Le titre `id`.
pub fn title(id: u32) -> Option<&'static Title> {
    CATALOG.iter().find(|t| t.id == id)
}

/// Un générateur pseudo-aléatoire (splitmix64) : rapide, sans dépendance, et identique
/// sur le serveur et dans le navigateur pour une même graine.
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

/// Tire `n` titres au hasard, sans doublon, parmi ceux de `platform` et `kind`
/// (`None` : tous). La même graine donne toujours le même tirage.
pub fn picks(
    seed: u64,
    platform: Option<Platform>,
    kind: Option<Kind>,
    n: usize,
) -> Vec<&'static Title> {
    let mut pool: Vec<&'static Title> = CATALOG
        .iter()
        .filter(|t| platform.is_none_or(|p| t.platform == p))
        .filter(|t| kind.is_none_or(|k| t.kind == k))
        .collect();
    let mut rng = Rng::new(seed ^ 0x005E_ED0F_F11E);
    let n = n.min(pool.len());
    // Fisher-Yates partiel : seuls les `n` premiers sont mélangés.
    for i in 0..n {
        let j = i + rng.below(pool.len() - i);
        pool.swap(i, j);
    }
    pool.truncate(n);
    pool
}

/// La similarité de deux titres (0 à 1) : genres en commun (indice de Jaccard), un
/// peu plus pour le même format et la même plateforme.
pub fn similarity(a: &Title, b: &Title) -> f32 {
    let (ma, mb) = (a.genre_mask(), b.genre_mask());
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
    if a.platform == b.platform {
        score += 0.08;
    }
    score
}

/// Pourquoi un titre est suggéré.
#[derive(Clone, Debug, PartialEq)]
pub enum Reason {
    /// Il ressemble à un titre aimé.
    BecauseYouLiked(&'static Title),
    /// Il est dans les genres préférés.
    YourGenres(Vec<Genre>),
    /// Une valeur sûre du catalogue (pas encore assez de votes).
    Classic,
}

impl Reason {
    /// La phrase affichée.
    pub fn text(&self) -> String {
        match self {
            Reason::BecauseYouLiked(t) => format!("Parce que vous avez aimé « {} »", t.name),
            Reason::YourGenres(genres) => format!(
                "Dans vos genres : {}",
                genres
                    .iter()
                    .map(|g| g.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Reason::Classic => "Une valeur sûre du catalogue".to_owned(),
        }
    }
}

/// Une suggestion.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    /// Le titre suggéré.
    pub title: &'static Title,
    /// Son score (plus il est haut, mieux c'est).
    pub score: f32,
    /// Pourquoi.
    pub reason: Reason,
}

/// Les valeurs sûres, montrées tant qu'il n'y a ni vote ni préférence.
const CLASSICS: [u32; 12] = [27, 8, 44, 6, 42, 21, 49, 9, 25, 15, 60, 32];

/// Les goûts d'une personne, tels que le moteur les lit.
#[derive(Clone, Debug, Default)]
pub struct Profile {
    /// Les titres aimés.
    pub liked: Vec<u32>,
    /// Les titres vus mais pas aimés.
    pub disliked: Vec<u32>,
    /// Les plateformes auxquelles elle est abonnée (vide : toutes).
    pub platforms: Vec<Platform>,
    /// Ses genres préférés.
    pub genres: Vec<Genre>,
    /// Films, séries ou les deux (`None`).
    pub kind: Option<Kind>,
}

/// Les `n` meilleures suggestions pour `profile`, parmi les titres pas encore votés.
pub fn recommend(profile: &Profile, n: usize) -> Vec<Suggestion> {
    let liked: Vec<&Title> = profile.liked.iter().filter_map(|&id| title(id)).collect();
    let disliked: Vec<&Title> = profile
        .disliked
        .iter()
        .filter_map(|&id| title(id))
        .collect();
    let preferred = profile.genres.iter().fold(0u16, |m, g| m | g.bit());

    let mut found: Vec<Suggestion> = CATALOG
        .iter()
        .filter(|t| !profile.liked.contains(&t.id) && !profile.disliked.contains(&t.id))
        .filter(|t| profile.platforms.is_empty() || profile.platforms.contains(&t.platform))
        .filter(|t| profile.kind.is_none_or(|k| t.kind == k))
        .map(|t| {
            // Le titre aimé qui lui ressemble le plus.
            let best = liked
                .iter()
                .map(|l| (similarity(t, l), *l))
                .max_by(|a, b| a.0.total_cmp(&b.0));
            let from_likes: f32 = liked.iter().map(|l| similarity(t, l)).sum();
            let from_dislikes: f32 = disliked.iter().map(|d| similarity(t, d)).sum();
            let shared = t.genre_mask() & preferred;
            let from_genres = shared.count_ones() as f32 * 0.6;
            let classic = CLASSICS
                .iter()
                .position(|&id| id == t.id)
                .map_or(0.0, |rank| 0.3 - rank as f32 * 0.02);
            let score = from_likes * 1.5 - from_dislikes + from_genres + classic;
            let reason = match best {
                Some((s, l)) if s >= 0.35 => Reason::BecauseYouLiked(l),
                _ if shared != 0 => Reason::YourGenres(
                    t.genres
                        .iter()
                        .copied()
                        .filter(|g| shared & g.bit() != 0)
                        .collect(),
                ),
                _ => Reason::Classic,
            };
            Suggestion {
                title: t,
                score,
                reason,
            }
        })
        .collect();
    found.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.title.id.cmp(&b.title.id))
    });
    found.truncate(n);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent() {
        assert_eq!(CATALOG.len(), 60);
        for (i, t) in CATALOG.iter().enumerate() {
            assert_eq!(t.id as usize, i + 1, "ids follow each other");
            assert!(!t.genres.is_empty() && t.hue < 360 && !t.pitch.is_empty());
        }
        for p in Platform::ALL {
            assert_eq!(CATALOG.iter().filter(|t| t.platform == p).count(), 20);
            assert_eq!(Platform::from_slug(p.slug()), Some(p));
        }
        for g in Genre::ALL {
            assert_eq!(Genre::from_slug(g.slug()), Some(g));
        }
    }

    #[test]
    fn picks_are_random_but_reproducible() {
        let a = picks(42, None, None, PICKS);
        assert_eq!(a.len(), 5);
        // La même valeur est vérifiée par les tests de l'application iOS.
        let ids: Vec<u32> = a.iter().map(|t| t.id).collect();
        assert_eq!(ids, [48, 58, 46, 54, 29]);
        assert_eq!(a, picks(42, None, None, PICKS));
        assert_ne!(a, picks(43, None, None, PICKS));
        let mut ids: Vec<u32> = a.iter().map(|t| t.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), 5, "no duplicates");
        let disney = picks(7, Some(Platform::DisneyPlus), Some(Kind::Film), PICKS);
        assert!(
            disney
                .iter()
                .all(|t| t.platform == Platform::DisneyPlus && t.kind == Kind::Film)
        );
    }

    #[test]
    fn suggestions_follow_the_votes() {
        let none = recommend(&Profile::default(), 4);
        assert_eq!(none[0].title.id, CLASSICS[0]);
        assert_eq!(none[0].reason, Reason::Classic);

        // Aimer des séries policières fait remonter des séries policières.
        let profile = Profile {
            liked: vec![25, 26],
            ..Profile::default()
        };
        let found = recommend(&profile, 3);
        assert!(found.iter().all(|s| s.title.genres.contains(&Genre::Crime)));
        assert!(matches!(found[0].reason, Reason::BecauseYouLiked(_)));
        assert!(found.iter().all(|s| s.title.id != 25 && s.title.id != 26));

        let filtered = recommend(
            &Profile {
                platforms: vec![Platform::Netflix],
                kind: Some(Kind::Film),
                genres: vec![Genre::Animation],
                ..Profile::default()
            },
            2,
        );
        assert_eq!(filtered[0].title.id, 20);
        assert!(matches!(filtered[0].reason, Reason::YourGenres(_)));
    }
}
