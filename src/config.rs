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
use std::hash::{Hash, Hasher};
use std::str::FromStr;

const DEFAULT_NAME: &str = "%%DEFAULT%%";

#[derive(PartialEq, Eq)]
pub struct KeyBind(pub KeyEvent);

impl fmt::Display for KeyBind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            KeyEvent::Char('\n') => {
                fmt.write_str(&format!("<CR>")).unwrap();
            }
            KeyEvent::Char('\t') => {
                fmt.write_str(&format!("<Tab>")).unwrap();
            }
            KeyEvent::Char(c) => {
                fmt.write_str(&format!("{}", c)).unwrap();
            }
            special => {
                fmt.write_str("<").unwrap();
                match special {
                    KeyEvent::Ctrl(c) => {
                        fmt.write_str(&format!("C-{}", c)).unwrap();
                    }
                    KeyEvent::Alt(c) => {
                        fmt.write_str(&format!("M-{}", c)).unwrap();
                    }
                    KeyEvent::F(n) => {
                        fmt.write_str(&format!("F{}", n)).unwrap();
                    }
                    x => {
                        fmt.write_str(&format!("{:?}", x)).unwrap();
                    }
                }
                fmt.write_str(">").unwrap();
            }
        }
        Ok(())
    }
}
impl FromStr for KeyBind {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(KeyBind(KeyEvent::Null));
        }
        if s.len() == 1 {
            return Ok(KeyBind(KeyEvent::Char(s.chars().next().unwrap())));
        }
        if s.starts_with('<') {
            let inside = s.trim_matches(|p| p == '<' || p == '>');

            if inside.to_lowercase().starts_with('f') {
                return Ok(KeyBind(KeyEvent::F(u8::from_str(&inside[1..]).or(Err(
                    Self::Err::new(&format!(
                        "Could not parse '{}' as a function key (e.g. <F12>)",
                        s
                    )),
                ))?)));
            }
            if inside.contains('-') {
                // it's a control character combo
                let parts: Vec<String> = inside
                    .split('-')
                    .map(|x| x.to_string())
                    .collect();
                let control_char = &parts[0].to_lowercase(); //.and_then(|z| Some(z.to_lowercase().as_str()));
                match control_char.as_ref() {
                    "c" => {
                        return Ok(KeyBind(KeyEvent::Ctrl(
                            parts
                                .get(1)
                                .and_then(|x| x.chars().next())
                                .ok_or(Self::Err::new(&format!(
                                    "Could not parse '{}' as a ctrl combo (e.g. <c-x>)",
                                    s
                                )))?,
                        )));
                    }
                    "m" | "a" => {
                        return Ok(KeyBind(KeyEvent::Alt(
                            parts
                                .get(1)
                                .and_then(|x| x.chars().next())
                                .ok_or(Self::Err::new(&format!(
                                    "Could not parse '{}' as a meta combo (e.g. <m-x> or <a-x>)",
                                    s
                                )))?,
                        )));
                    }
                    _ => {
                        return Err(Self::Err::new(&format!(
                            "Unrecognized control char in '{}'",
                            s
                        )))
                    }
                }
            } else {
                // it's just a special char
                return Ok(KeyBind(match inside.to_lowercase().as_ref() {
                    "bs" | "backspace" => Ok(KeyEvent::Backspace),
                    "enter" | "cr" => Ok(KeyEvent::Char('\n')),
                    "tab" => Ok(KeyEvent::Char('\t')),
                    "backtab" => Ok(KeyEvent::BackTab),
                    "esc" => Ok(KeyEvent::Esc),
                    "up" => Ok(KeyEvent::Up),
                    "down" => Ok(KeyEvent::Down),
                    "left" => Ok(KeyEvent::Left),
                    "right" => Ok(KeyEvent::Right),
                    "home" => Ok(KeyEvent::Home),
                    "end" => Ok(KeyEvent::End),
                    "pageup" => Ok(KeyEvent::PageUp),
                    "pagedown" => Ok(KeyEvent::PageDown),
                    "delete" | "del" => Ok(KeyEvent::Delete),
                    "insert" => Ok(KeyEvent::Insert),
                    "null" => Ok(KeyEvent::Null),
                    _ => Err(Self::Err::new(&format!("Unrecognized special key: {}", s))),
                }?));
            }
        }
        Err(Self::Err::new(&format!(
            "Unrecognized keymap format: {}",
            s
        )))
    }
}
impl<'de> Deserialize<'de> for KeyBind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}
impl Serialize for KeyBind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}
impl Hash for KeyBind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_string().hash(state);
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub engines: HashMap<String, Engine>,
    pub keybinds: HashMap<KeyBind, BindableAction>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum BindableAction {
    SelectNext,
    SelectPrev,
    DeleteWord,
    DeleteChar,
    Exit,
    Submit,
    ClearInput,
    AddChar(char),
}

#[derive(Debug)]
pub struct ConfigError {
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
    .map_err(|e| {
        ConfigError::new(&format!(
            " ↳ Could not parse TOML file {:?}:\n    ↳ {}",
            config_path, e
        ))
    })
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
            println!(
                "{}{}Error!{} Could not load config:\n{}\nUsing default config.",
                Attribute::Bold,
                Colored::Fg(Color::Red),
                Attribute::Reset,
                e
            );
            get_default_config()
        }
    };
    validate_config(&mut config);
    config
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
        "ddg".to_string(),
        Engine {
            name: "DuckDuckGo".to_string(),
            suggestion_url: "https://duckduckgo.com/ac/?q=%s&type=list".to_string(),
            search_url: "https://duckduckgo.com/?q=%s".to_string(),
            space_becomes: "+".into(),
            prompt: Prompt {
                icon_fg: Color::White,
                icon_bg: Color::Rgb {
                    r: 222,
                    g: 88,
                    b: 51,
                },
                icon: String::from(" ♞ "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: DEFAULT_NAME.into()
            },
        },
    );
    engs.insert(
        "g".to_string(),
        Engine {
            name: "Google (I'm Feeling Lucky)".to_string(),
            suggestion_url: "https://www.google.com/complete/search?client=chrome&q=%s".to_string(),
            search_url: "https://www.google.com/search?btnI&q=%s".to_string(),
            space_becomes: "+".into(),
            prompt: Prompt {
                icon_fg: Color::White,
                icon_bg: Color::Blue,
                icon: String::from(" g "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: " I'm Feeling Lucky ".into(),
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
                text: DEFAULT_NAME.into(),
            },
        },
    );
    engs.insert(
        "wkt".to_string(),
        Engine {
            name: "Wiktionary".to_string(),
            suggestion_url: "https://en.wiktionary.org/w/api.php?action=opensearch&search=%s&limit=15&namespace=0&format=json".to_string(),
            search_url: "https://www.wiktionary.org/search-redirect.php?family=wiktionary&language=en&search=%s&go=Go".to_string(),
            space_becomes: "+".into(),
            prompt: Prompt {
                icon_fg: Color::Black,
                icon_bg: Color::White,
                icon: String::from("['w]"),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: DEFAULT_NAME.into()
            },
        },
    );
    engs.insert(
        "w".to_string(),
        Engine {
            name: "Wikipedia".to_string(),
            suggestion_url: "https://en.wikipedia.org/w/api.php?action=opensearch&search=%s&limit=15&namespace=0&format=json".to_string(),
            search_url: "https://www.wikipedia.org/search-redirect.php?family=wikipedia&language=en&search=%s&language=en&go=Go".to_string(),
            space_becomes: "+".into(),
            prompt: Prompt {
                icon_fg: Color::Black,
                icon_bg: Color::White,
                icon: String::from(" W "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: DEFAULT_NAME.into()
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
                text: DEFAULT_NAME.into(),
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
                text: DEFAULT_NAME.into(),
            },
        },
    );
    let mut keybinds = HashMap::new();

    keybinds.insert(KeyBind(KeyEvent::Ctrl('c')), BindableAction::Exit);
    keybinds.insert(KeyBind(KeyEvent::Esc), BindableAction::Exit);
    keybinds.insert(KeyBind(KeyEvent::Char('\n')), BindableAction::Submit);
    keybinds.insert(KeyBind(KeyEvent::Ctrl('w')), BindableAction::DeleteWord);
    keybinds.insert(KeyBind(KeyEvent::Ctrl('n')), BindableAction::SelectNext);
    keybinds.insert(KeyBind(KeyEvent::Char('\t')), BindableAction::SelectNext);
    keybinds.insert(KeyBind(KeyEvent::Down), BindableAction::SelectNext);
    keybinds.insert(KeyBind(KeyEvent::Ctrl('p')), BindableAction::SelectPrev);
    keybinds.insert(KeyBind(KeyEvent::BackTab), BindableAction::SelectPrev);
    keybinds.insert(KeyBind(KeyEvent::Up), BindableAction::SelectPrev);
    keybinds.insert(KeyBind(KeyEvent::Backspace), BindableAction::DeleteChar);

    Config {
        engines: engs,
        keybinds,
    }
}
pub fn serialize_color<S>(x: &Color, s: S) -> Result<S::Ok, S::Error>
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

pub fn deserialize_color<'de, D>(d: D) -> Result<Color, D::Error>
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
