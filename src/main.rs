extern crate crossterm;
extern crate json;
extern crate reqwest;
extern crate webbrowser;

use crossterm::{
    input, Attribute, ClearType, Color, Colored, Crossterm, InputEvent, KeyEvent, RawScreen,
};

use std::cmp::min;
use std::collections::HashMap;
use std::result::Result;
use std::sync::mpsc;
use std::thread;

#[allow(unused_must_use)]
fn main() {
    let (tx, rx) = mpsc::channel();

    let _screen = RawScreen::into_raw_mode();
    let crossterm = Crossterm::new();
    let mut cursor = crossterm.cursor();
    let terminal = crossterm.terminal();

    let input = input();
    let mut stdin = input.read_sync();
    let mut input_buf = String::new();

    let engines = define_engines();
    let input_tx = tx.clone();
    let input_thread = thread::spawn(move || {
        let mut should_quit = false;
        loop {
            let key = stdin.next();
            match &key {
                Some(ie) => match ie {
                    InputEvent::Keyboard(k) => match k {
                        KeyEvent::Char('\n') => {
                            should_quit = true;
                        }
                        KeyEvent::Ctrl(c) => match c {
                            'c' => {
                                should_quit = true;
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            };
            input_tx.send(UiMsg::OnInput(key));
            if should_quit {
                break;
            }
        }
    });

    cursor.hide();
    let mut input_line: String = String::from("");
    let mut suggs: Option<Suggestions> = None;
    let mut prompt = &engines.get("").expect("No default engine set.").prompt;
    let mut waiting_for_term: Option<String> = None; //
    let (t_w, _t_h) = terminal.terminal_size();
    let mut selected_n: Option<usize> = None;

    let mut refresh_completions = true;
    loop {
        let (engine, prefix, search_term) = match_engine(&input_line, &engines);
        if refresh_completions {
            prompt = &engine.prompt;
            if input_line.is_empty() {
                suggs = None;
            } else {
                waiting_for_term = Some(search_term.clone());
                let url = engine.format_suggestion_url(&search_term);
                let tx2 = tx.clone();
                thread::spawn(move || {
                    if let Some(resolved_suggs) = fetch_suggs(url).ok() {
                        tx2.send(UiMsg::SetSuggestions(resolved_suggs));
                    }
                });
            }
            refresh_completions = false;
        }
        cursor.move_left(t_w);
        terminal.clear(ClearType::CurrentLine);
        print!(
            "{}{}{}{}",
            Colored::Fg(prompt.icon_fg),
            Colored::Bg(prompt.icon_bg),
            prompt.icon,
            Attribute::Reset
        ); // icon
        print!(
            "{}{}{}{}",
            Colored::Fg(prompt.text_fg),
            Colored::Bg(prompt.text_bg),
            prompt.text,
            Attribute::Reset
        ); // promptt text
        println!(" {}_", input_line);
        let suggest_lines = 15; /*if let Some(ref suggs) = suggs {
                                    suggs.sugg_terms.len()
                                } else {
                                    0
                                }; */
        let selectable_lines = if let Some(ref suggs) = suggs { min(suggest_lines, suggs.sugg_terms.len())} else {0};
        for n in 0..suggest_lines {
            terminal.clear(ClearType::CurrentLine);
            cursor.move_left(t_w);

            if let Some(ref suggs) = suggs {
                match suggs.sugg_terms.get(n) {
                    Some(line) => match selected_n {
                        Some(selected_n) if selected_n == n => {
                            print!(
                                "{}{}{}{}",
                                Colored::Fg(Color::Black),
                                Colored::Bg(Color::White),
                                line,
                                Attribute::Reset
                            );
                        }
                        _ => {
                            print!("{}", line);
                        }
                    },
                    None => {} // outside sugg bounds
                }
            }

            println!();
        }
        let msg = rx.recv().unwrap();
        match msg {
            UiMsg::SetSuggestions(suggestion_update) => {
                if let Some(ref expected_term) = waiting_for_term {
                    if &suggestion_update.term == expected_term {
                        suggs = Some(suggestion_update);
                    }
                }
            }
            UiMsg::OnInput(key) => {
                refresh_completions = true;
                match key {
                    Some(ie) => match ie {
                        InputEvent::Keyboard(k) => match k {
                            KeyEvent::Char('\n') => {
                                input_buf.clear();
                                let url = engine.format_search_url(&search_term);
                                for _ in 0..suggest_lines {
                                    cursor.move_up(1);
                                    terminal.clear(ClearType::CurrentLine);
                                }
                                cursor.move_left(t_w);
                                println!("Opening {}", url);
                                terminal.clear(ClearType::CurrentLine);
                                webbrowser::open(&url).expect("Couldn't open browser.");
                                break;
                            }
                            // TODO: clean up brutal code reuse here
                            KeyEvent::Char('\t') | KeyEvent::Ctrl('n') | KeyEvent::Down => {
                                if let Some(ref suggs) = suggs {
                                    selected_n = Some(if let Some(selected_n) = selected_n {
                                        selected_n + 1
                                    } else {
                                        0
                                    });
                                    if selected_n.unwrap()
                                        >= selectable_lines
                                    {
                                        selected_n = Some(0);
                                    }
                                    if let Some(selected) =
                                        suggs.sugg_terms.get(selected_n.unwrap())
                                    {
                                        let (_, interfering_prefix, _) =
                                            match_engine(&selected, &engines);
                                        if !interfering_prefix.is_empty() {
                                            input_line = format!("?{}", selected.to_string());
                                        } else if !prefix.is_empty() {
                                            input_line =
                                                format!("{} {}", prefix, selected.to_string());
                                        } else {
                                            input_line = format!("{}", selected.to_string());
                                        }
                                        refresh_completions = false;
                                    }
                                }
                            }
                            KeyEvent::BackTab | KeyEvent::Ctrl('p') | KeyEvent::Up => {
                                if let Some(ref suggs) = suggs {
                                    selected_n =
                                        Some(selected_n.unwrap_or(0).checked_sub(1).unwrap_or(
                                           selectable_lines.checked_sub(1).unwrap_or(0),
                                        ));
                                    if let Some(selected) =
                                        suggs.sugg_terms.get(selected_n.unwrap())
                                    {
                                        let (_, interfering_prefix, _) =
                                            match_engine(&selected, &engines);
                                        if !interfering_prefix.is_empty() {
                                            input_line = format!("?{}", selected.to_string());
                                        } else if !prefix.is_empty() {
                                            input_line =
                                                format!("{} {}", prefix, selected.to_string());
                                        } else {
                                            input_line = format!("{}", selected.to_string());
                                        }
                                        refresh_completions = false;
                                    }
                                }
                            }
                            KeyEvent::Char(character) => {
                                input_line.push(character as char);
                                selected_n = None;
                            }

                            KeyEvent::Backspace => {
                                input_line.pop();
                                selected_n = None;
                            }
                            KeyEvent::Ctrl(c) => match c {
                                'c' => {
                                    terminal.clear(ClearType::CurrentLine);
                                    for _ in 0..suggest_lines {
                                        cursor.move_up(1);
                                        terminal.clear(ClearType::CurrentLine);
                                    }
                                    break;
                                }
                                'w' => {
                                    // delete last word (trim, then delete backwards until first
                                    // word character or beginning of line
                                    input_line = input_line
                                        .trim_end()
                                        .trim_end_matches(|x: char| !x.is_whitespace())
                                        .to_string();
                                }
                                _ => {}
                            },
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                }
            }
        };
        cursor.move_up(suggest_lines as u16 + 1);
    }
    cursor.show();
    cursor.move_left(t_w);

    input_thread.join();
}

fn fetch_suggs(url: String) -> Result<Suggestions, Box<std::error::Error>> {
    let text = reqwest::get(&url)?.text()?;
    let data = json::parse(&text)?;
    let term = data[0]
        .as_str()
        .ok_or("first array value not a string")?
        .to_string();
    let sugg_terms: Vec<String> = data[1].members().map(|opt|opt.as_str().expect("one of the values in the second value (which should be an array) is not a string").to_string()).collect(); // todo: error handling
    Ok(Suggestions { term, sugg_terms })
}

#[allow(dead_code)] // maybe I'll use name later ok
struct Engine {
    prompt: Prompt,
    name: String,
    suggestion_url: String,
    search_url: String,
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
fn match_engine<'a, 'b>(
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

fn define_engines() -> HashMap<String, Engine> {
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
// mirrors opensearch schema
#[derive(Debug)]
struct Suggestions {
    term: String,
    sugg_terms: Vec<String>,
    //descriptions: Vec<String>,
    //urls: Vec<String>
}

#[derive(Clone)]
struct Prompt {
    icon_fg: Color,
    icon_bg: Color,
    icon: String,
    text_fg: Color,
    text_bg: Color,
    text: String,
}

enum UiMsg {
    SetSuggestions(Suggestions),
    OnInput(Option<InputEvent>),
}
