/// The Classical California Stations.
use clap::ValueEnum;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Station {
    pub name: &'static str,
    pub search: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Default, Copy, Clone, ValueEnum)]
pub enum ClassicalStations {
    #[default]
    #[value(alias = "cc")]
    ClassicalCalifornia,
    #[value(alias = "ul")]
    Ulitmate,
    #[value(alias = "ge")]
    GreatEscape,
    #[value(alias = "nm-en")]
    NuestraMusicaEn,
    #[value(alias = "nm-es")]
    NuestraMusicaEs,
    #[value(alias = "ar")]
    Arcade,
    #[value(alias = "am")]
    Americana,
    #[value(alias = "xmas")]
    Christmas,
    #[value(alias = "gl")]
    Glissando,
}

impl ClassicalStations {
    pub fn station(&self) -> Station {
        match self {
            Self::ClassicalCalifornia => Station {
                name: "Classical California",
                search: "Classical KUSC",
                description: "Live Streaming 24/7 Classical Music",
            },
            Self::Ulitmate => Station {
                name: "Ultimate Playlist",
                search: "KDFC Classical California Ultimate Playlist",
                description: "Listener-voted favorites",
            },
            Self::GreatEscape => Station {
                name: "Great Escape",
                search: "KDFC Great Escape",
                description: "Peaceful, ambient classical",
            },
            Self::NuestraMusicaEn => Station {
                name: "Nuestra Música [In English]",
                search: "CC - Nuestra Música [In English]",
                description: "Classical music from Latin composers",
            },
            Self::NuestraMusicaEs => Station {
                name: "Nuestra Música [En Español]",
                search: "CC - Nuestra Música [En Español]",
                description: "Classical music from Latin composers",
            },
            Self::Arcade => Station {
                name: "Arcade",
                search: "KDFC Arcade",
                description: "Video game & film scores",
            },
            Self::Americana => Station {
                name: "Americana",
                search: "KDFC Classical Americana",
                description: "American classical composers",
            },
            Self::Christmas => Station {
                name: "Classical Christmas",
                search: "KDFC Classical Christmas",
                description: "Christmas Classics",
            },
            Self::Glissando => Station {
                name: "Glissando",
                search: "KDFC Glissando",
                description: "A New Children's Musical Adventure Every 20 Minutes!",
            },
        }
    }
}
