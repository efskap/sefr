use crate::*;

#[allow(dead_code)] // maybe I'll use name later ok
pub struct Engine {
    pub prompt: Prompt,
    pub name: String,
    pub suggestion_url: String,
    pub search_url: String,
}

#[derive(Clone)]
pub struct Prompt {
    pub icon_fg: Color,
    pub icon_bg: Color,
    pub icon: String,
    pub text_fg: Color,
    pub text_bg: Color,
    pub text: String,
}
impl Engine {
    pub fn format_suggestion_url(&self, search_term: &str) -> String {
        self.suggestion_url
            .replace("%s", &search_term.replace(" ", "+"))
    }
    pub fn format_search_url(&self, search_term: &str) -> String {
        self.search_url
            .replace("%s", &search_term.replace(" ", "+"))
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
pub fn define_engines() -> HashMap<String, Engine> {
    let mut engs = HashMap::new();
    engs.insert(
        "".to_string(),
        Engine {
            name: "Google".to_string(),
            suggestion_url: "https://www.google.com/complete/search?client=chrome&q=%s".to_string(),
            search_url: "https://www.google.com/search?q=%s".to_string(),
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
            prompt: Prompt {
                icon_fg: Color::White,
                icon_bg: Color::Red,
                icon: String::from(" ⬬ "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: String::from(" Reddit "),
            },
        },
    );
    engs.insert(
        "wkt".to_string(),
        Engine {
            name: "Wiktionary".to_string(),
            suggestion_url: "https://en.wiktionary.org/w/api.php?action=opensearch&search=%s&limit=10&namespace=0&format=json".to_string(),
            search_url: "https://en.wiktionary.org/wiki/%s".to_string(),
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
            prompt: Prompt {
                icon_fg: Color::White,
                icon_bg: Color::Red,
                icon: String::from(" ▶ "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: String::from(" YouTube "),
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
            prompt: Prompt {
                icon_fg: Color::White,
                icon_bg: Color::Red,
                icon: String::from(" ⬬ "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: String::from(" Subreddit "),
            },
        },
    );
    engs
}
