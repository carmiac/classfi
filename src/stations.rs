/// The Classical California Stations. Uses https://www.radio-browser.info to get the current streaming URL.
use radiobrowser::RadioBrowserAPI;
use url::Url;

struct StationDef {
    name: &'static str,
    description: &'static str,
}

const STATION_DEFS: &[StationDef] = &[
    StationDef {
        name: "Classical KUSC",
        description: "Classical California - 24/7 Classical Music",
    },
    StationDef {
        name: "KDFC Classical California Ultimate Playlist",
        description: "Listener-voted favorites",
    },
    StationDef {
        name: "KDFC Great Escape",
        description: "Peaceful, ambient classical",
    },
    StationDef {
        name: "CC - Nuestra Música [In English]",
        description: "Classical music from Latin composers",
    },
    StationDef {
        name: "KDFC Arcade",
        description: "Video game & film scores",
    },
    StationDef {
        name: "KDFC Classical Americana",
        description: "American classical composers",
    },
    StationDef {
        name: "KDFC Classical Christmas",
        description: "Christmas Classics",
    },
    StationDef {
        name: "KDFC Glissando",
        description: "A New Children's Musical Adventure Every 20 Minutes!",
    },
];

#[derive(Debug, Clone)]
pub struct Station {
    pub name: &'static str,
    pub description: &'static str,
    pub url: Option<Url>,
}

impl Station {
    /// Get the stream URL if available and cache it.
    pub async fn get_url(&mut self) -> Option<Url> {
        if self.url.is_none() {
            info!("Getting URL for {}", self.name);
            if let Ok(api) = RadioBrowserAPI::new().await.map_err(|e| e.to_string())
                && let Ok(stations) = api
                    .get_stations()
                    .name(self.name)
                    .name_exact(true)
                    .send()
                    .await
                    .map_err(|e| e.to_string())
                && let Some(s) = stations.into_iter().next()
            {
                info!("Got URL: {}", s.url_resolved);
                self.url = Url::parse(s.url_resolved.as_str()).ok();
            }
        }
        self.url.clone()
    }

    pub fn all() -> Vec<Self> {
        STATION_DEFS
            .iter()
            .map(|def| Station {
                name: def.name,
                description: def.description,
                url: None,
            })
            .collect()
    }
}
