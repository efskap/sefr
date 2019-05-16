// *************************************************************************
// * Copyright (C) 2019 Dmitry Narkevich (me@dmitry.lol)                   *
// *                                                                       *
// * This program is free software: you can redistribute it and/or modify  *
// * it under the terms of the GNU General Public License as published by  *
// * the Free Software Foundation, either version 3 of the License, or     *
// * (at your option) any later version.                                   *
// *                                                                       *
// * This program is distributed in the hope that it will be useful,       *
// * but WITHOUT ANY WARRANTY; without even the implied warranty of        *
// * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
// * GNU General Public License for more details.                          *
// *                                                                       *
// * You should have received a copy of the GNU General Public License     *
// * along with this program.  If not, see <http://www.gnu.org/licenses/>. *
// *************************************************************************

use crate::*;

use directories::ProjectDirs;
use serde::de::SeqAccess;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::error::Error;
use std::fmt;
use std::fs;
use std::str::FromStr;

const DEFAULT_NAME: &str = "%%DEFAULT%%";

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub engines: HashMap<String, Engine>,
}

#[derive(Serialize, Deserialize)]
pub struct Engine {
    #[serde(default = "default_prompt")]
    pub prompt: Prompt,
    pub name: String,
    #[serde(default)]
    pub suggestion_url: String,
    pub search_url: String,
    #[serde(default = "_default_space_becomes", skip_serializing_if = "_is_default_space_becomes")]
    pub space_becomes: String,
}

impl Engine {
    pub fn format_suggestion_url(&self, search_term: &str) -> String {
        self.suggestion_url
            .replace("%s", &search_term.replace(" ", "+"))
    }
    pub fn format_search_url(&self, search_term: &str) -> String {
        self.search_url
            .replace("%s", &search_term.replace(" ", &self.space_becomes))
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Prompt {
    #[serde(
        deserialize_with = "deserialize_color",
        serialize_with = "serialize_color",
        default = "_white"
    )]
    pub icon_fg: Color,
    #[serde(
        deserialize_with = "deserialize_color",
        serialize_with = "serialize_color",
        default = "_black"
    )]
    pub icon_bg: Color,
    #[serde(default)]
    pub icon: String,
    #[serde(
        deserialize_with = "deserialize_color",
        serialize_with = "serialize_color",
        default = "_black"
    )]
    pub text_fg: Color,
    #[serde(
        deserialize_with = "deserialize_color",
        serialize_with = "serialize_color",
        default = "_white"
    )]
    pub text_bg: Color,
    // TODO: figure out how Option is serialized instead of this heck
    #[serde(default = "_default_name", skip_serializing_if = "_is_default_name")]
    pub text: String,
}
fn _default_name() -> String {
    DEFAULT_NAME.into()
}
fn _is_default_name(s: &str) -> bool {
    s == _default_name()
}
fn _default_space_becomes() -> String {
    "+".into()
}
fn _is_default_space_becomes(s: &str) -> bool {
    s == _default_space_becomes()
}
fn _white() -> Color {
    Color::White
}
fn _black() -> Color {
    Color::Black
}
fn serialize_color<S>(x: &Color, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match x {
        Color::Rgb { r, g, b } => {
            let mut state = s.serialize_seq(Some(3))?;
            state.serialize_element(r)?;
            state.serialize_element(g)?;
            state.serialize_element(b)?;
            state.end()
        }
        Color::AnsiValue(v) => s.serialize_u8(*v),
        x => s.serialize_str(&format!("{:?}", x)),
    }
}

fn deserialize_color<'de, D>(d: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    d.deserialize_any(ColorVisitor)
}

use serde::de::{self, Visitor};

struct ColorVisitor;

impl<'de> Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A string (name), u8 number (ansi color), or an [r,g,b] u8 array")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Color::from_str(value).or_else(|_| {
            Err(E::custom(format!(
                "Invalid color name: {}. See crossterm's Color enum for valid values.",
                value
            )))
        })
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Color::AnsiValue(value))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let r: u8 = match seq.next_element()? {
            Some(nth) => nth,
            None => {
                return Err(de::Error::custom(format!(
                    "R component invalid, should be a value 0-255."
                )));
            }
        };
        let g: u8 = match seq.next_element()? {
            Some(nth) => nth,
            None => {
                return Err(de::Error::custom(format!(
                    "G component invalid, should be a value 0-255."
                )));
            }
        };
        let b: u8 = match seq.next_element()? {
            Some(nth) => nth,
            None => {
                return Err(de::Error::custom(format!(
                    "B component invalid, should be a value 0-255."
                )));
            }
        };
        Ok(Color::Rgb { r, g, b })
    }
}
impl Prompt {
    pub fn draw(&self) {
        print!(
            "{}{}{}{}",
            Colored::Fg(self.icon_fg),
            Colored::Bg(self.icon_bg),
            self.icon,
            Attribute::Reset
        ); // icon
        print!(
            "{}{}{}{}",
            Colored::Fg(self.text_fg),
            Colored::Bg(self.text_bg),
            self.text,
            Attribute::Reset
        ); // promptt text
    }
}
// TODO: this should prolly return slices, not Strings
pub fn match_engine<'a, 'b>(
    input_line: &'b str,
    engines: &'a HashMap<String, Engine>,
) -> (&'a Engine, String, String) {
    let default_engine = engines.get("").unwrap();

    // escape search engine keyword with question mark like Chrome
    if input_line.starts_with("?") {
        return (default_engine, String::new(), input_line[1..].to_string());
    }

    let words: Vec<&str> = input_line.split_whitespace().collect();

    // in the empty case, or if a keyword is typed but there's no space after it, skip matching.
    if words.len() < 1 || (words.len() == 1 && !input_line.ends_with(" ")) {
        return (default_engine, String::new(), input_line.trim().to_string());
    }

    let potential_prefix = words.first().unwrap();
    match engines.get(&potential_prefix.to_string()) {
        Some(engine) => {
            let search_term = input_line[potential_prefix.len()..].trim().to_string();
            (engine, potential_prefix.to_string(), search_term)
        }
        None => (default_engine, String::new(), input_line.trim().to_string()),
    }
}

#[derive(Debug)]
struct ConfigError {
    details: String,
}

impl ConfigError {
    fn new(msg: &str) -> ConfigError {
        ConfigError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for ConfigError {
    fn description(&self) -> &str {
        &self.details
    }
}

fn load_config_from_file() -> Result<Config, ConfigError> {
    let proj_dirs = ProjectDirs::from("com", "efskap", "sefr")
        .ok_or(ConfigError::new("Couldn't get config dir."))?;
    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir).or(Err(ConfigError::new(&format!(
        "Couldn't create dir {:?}.",
        config_dir
    ))))?;
    let config_path = config_dir.join("config.toml");
    let default_toml = format!(
        "{}",
        toml::Value::try_from(&get_default_config()).or(Err(ConfigError::new(
            "Could not serialize default config to TOML. This... shouldn't happen."
        )))?
    );
    if !config_path.exists() {
        fs::write(&config_path, default_toml).or(Err(ConfigError::new(&format!(
            "Could not write default config to {:?}.",
            config_path
        ))))?;
        println!(
            "Wrote default config to {:?}. Edit it and enjoy!",
            config_path
        );
    }
    toml::from_str(
        &fs::read_to_string(&config_path).or(Err(ConfigError::new(&format!(
            "Could not read path {:?}.",
            config_path
        ))))?,
    )
    .or(Err(ConfigError::new(&format!("Could not parse TOML file {:?}.", config_path))))
}

fn validate_config(config: &mut Config) {
    let default = config
        .engines
        .remove("_default")
        .expect("No '_default' search engine found!!!");
    config.engines.insert("".to_string(), default);

    // first fix em up
    for eng in config.engines.values_mut() {
        // if user didn't specify prompt text, just set it to the engine name with padding
        // of course this won't happen if they set it to an empty string.
        if eng.prompt.text == DEFAULT_NAME {
            eng.prompt.text = format!(" {} ", eng.name);
        }
    }
    // then get rid of invalid ones
    let bad_prefixes: Vec<String>  = config.engines
        .iter().filter_map(|(k,v)| {
            if k.contains(' ') {
                println!("Prefixes have to be a single word, so engine '{}' with prefix '{}' will be ignored.", v.name, k);
                return Some(k.clone());
            }
            None
        }).collect();

    for prefix in bad_prefixes {
        config.engines.remove(&prefix);
    }
}

pub fn get_config() -> Config {
    if let Some(proj_dirs) = ProjectDirs::from("com", "efskap", "sefr") {
        proj_dirs.config_dir();
    }

    let mut config = match load_config_from_file() {
        Ok(x) => x,
        Err(e) => {
            println!("{}{}Error!{} Could not load config: {}\nUsing default config.", Attribute::Bold, Colored::Fg(Color::Red), Attribute::Reset,e);
            get_default_config()
        }
    };
    validate_config(&mut config);
    config
}

fn default_prompt() -> Prompt {
    Prompt {
        icon_fg: Color::White,
        icon_bg: Color::Blue,
        icon: String::from(" > "),
        text_fg: Color::Black,
        text_bg: Color::White,
        text: String::from(DEFAULT_NAME),
    }
}

fn get_default_config() -> Config {
    let mut engs = HashMap::new();
    engs.insert(
        "_default".to_string(),
        Engine {
            name: "Google".to_string(),
            suggestion_url: "https://www.google.com/complete/search?client=chrome&q=%s".to_string(),
            search_url: "https://www.google.com/search?q=%s".to_string(),
            space_becomes: "+".into(),
            prompt: Prompt {
                icon_fg: Color::White,
                icon_bg: Color::Blue,
                icon: String::from(" g "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: String::from(" Google "),
            },
        },
    );
    engs.insert(
        "red".to_string(),
        Engine {
            name: "Reddit".to_string(),
            suggestion_url: "https://www.google.com/complete/search?client=chrome&q=%s".to_string(),
            search_url: "https://www.google.com/search?q=site:reddit.com+%s".to_string(),
            space_becomes: "+".into(),
            prompt: Prompt {
                icon_fg: Color::White,
                icon_bg: Color::Rgb {
                    r: 255,
                    g: 69,
                    b: 0,
                },
                icon: String::from(" ⬬ "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: DEFAULT_NAME.to_string(),
            },
        },
    );
    engs.insert(
        "wkt".to_string(),
        Engine {
            name: "Wiktionary".to_string(),
            suggestion_url: "https://en.wiktionary.org/w/api.php?action=opensearch&search=%s&limit=10&namespace=0&format=json".to_string(),
            search_url: "https://en.wiktionary.org/wiki/%s".to_string(),
            space_becomes: "_".into(),
            prompt: Prompt {
                icon_fg: Color::Black,
                icon_bg: Color::White,
                icon: String::from(" W "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: String::from(" Wiktionary (en) "),
            },
        },
    );
    engs.insert(
        "yt".to_string(),
        Engine {
            name: "YouTube".to_string(),
            suggestion_url:
                "http://suggestqueries.google.com/complete/search?client=firefox&ds=yt&q=%s"
                    .to_string(),
            search_url: "https://www.youtube.com/results?q=%s".to_string(),
            space_becomes: "+".into(),
            prompt: Prompt {
                icon_fg: Color::White,
                icon_bg: Color::Red,
                icon: String::from(" ▶ "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: DEFAULT_NAME.to_string(),
            },
        },
    );
    engs.insert(
        "r".to_string(),
        Engine {
            name: "Subreddit".to_string(),
            suggestion_url:
                "https://us-central1-subreddit-suggestions.cloudfunctions.net/suggest?query=%s"
                    .to_string(),
            search_url: "https://www.reddit.com/r/%s".to_string(),
            space_becomes: "".into(), // subreddits dont have spaces
            prompt: Prompt {
                icon_fg: Color::White,
                icon_bg: Color::Rgb {
                    r: 255,
                    g: 69,
                    b: 0,
                },
                icon: String::from(" ⬬ "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: String::from(" Subreddit "),
            },
        },
    );
    Config { engines: engs }
}
