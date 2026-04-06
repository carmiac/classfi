/// The Classical California Stations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Station {
    pub name: &'static str,
    pub description: &'static str,
}

pub const CLASSICAL_STATIONS: &[Station] = &[
    Station {
        name: "Classical KUSC",
        description: "Classical California - 24/7 Classical Music",
    },
    Station {
        name: "KDFC Classical California Ultimate Playlist",
        description: "Listener-voted favorites",
    },
    Station {
        name: "KDFC Great Escape",
        description: "Peaceful, ambient classical",
    },
    Station {
        name: "CC - Nuestra Música [In English]",
        description: "Classical music from Latin composers",
    },
    Station {
        name: "CC - Nuestra Música [En Español]",
        description: "Classical music from Latin composers",
    },
    Station {
        name: "KDFC Arcade",
        description: "Video game & film scores",
    },
    Station {
        name: "KDFC Classical Americana",
        description: "American classical composers",
    },
    Station {
        name: "KDFC Classical Christmas",
        description: "Christmas Classics",
    },
    Station {
        name: "KDFC Glissando",
        description: "A New Children's Musical Adventure Every 20 Minutes!",
    },
];
