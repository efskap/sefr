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
use crate::config::*;

use std::fmt::{Formatter,Display};
use serde::{Deserialize, Serialize};
use percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};

const DEFAULT_NAME: &str = "%%DEFAULT%%";



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
    fn encode(s: &str) -> String {
        utf8_percent_encode(&s, DEFAULT_ENCODE_SET).to_string().replace("+", "%2B").into()
    }
    pub fn format_suggestion_url(&self, search_term: &str) -> String {
        self.suggestion_url
            .replace("%s", &Self::encode(&search_term))
    }
    pub fn format_search_url(&self, search_term: &str) -> String {
        self.search_url
            .replace("%s", &Self::encode(&search_term))
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
impl Prompt {
    pub fn to_short(&self) -> ShortPrompt {
        ShortPrompt(self)
    }

}
impl Display for Prompt {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
            write!(f,
            "{}{}{}{}",
            Colored::Fg(self.icon_fg),
            Colored::Bg(self.icon_bg),
            self.icon,
            Attribute::Reset
        )?; // icon
        write!(f,
            "{}{}{}{}",
            Colored::Fg(self.text_fg),
            Colored::Bg(self.text_bg),
            self.text,
            Attribute::Reset
        )?; // promptt text
        Ok(())
    }
}

pub struct ShortPrompt<'a> (&'a Prompt);

impl<'a> Display for ShortPrompt<'a> {
    fn fmt(& self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
            write!(f,
            "{}{}{}{}",
            Colored::Fg(self.0.icon_fg),
            Colored::Bg(self.0.icon_bg),
            self.0.icon,
            Attribute::Reset
        )?; // icon
        Ok(())
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
