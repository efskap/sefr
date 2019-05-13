extern crate crossterm;
extern crate reqwest;
//extern crate serde_json;
//extern crate serde;
extern crate json;

use crossterm::{
    cursor, input, terminal, AlternateScreen, Attribute, ClearType, Color, Colored, Crossterm,
    InputEvent, KeyEvent, RawScreen, Terminal, TerminalCursor,
};

use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::result::Result;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{process, thread, time};

//const SUGGEST_LINES: u16 = 10;

#[allow(unused_must_use)]
fn main() {
    let (tx, rx) = mpsc::channel();

    let screen = RawScreen::into_raw_mode();
    let crossterm = Crossterm::new();
    let cursor = crossterm.cursor();
    let terminal = crossterm.terminal();

    let input = input();
    let mut stdin = input.read_sync();
    let mut input_buf = String::new();

    let input_thread = thread::spawn(move || {
        let engines = define_engines();
        loop {
            let (engine, search_term) = match_engine(&input_buf, &engines);
            tx.send(UiMsg::SetInputLine(
                engine.prompt.clone(),
                input_buf.clone(),
            ));
            if !input_buf.is_empty() {
                tx.send(UiMsg::ExpectSuggsFor(search_term.clone()));
                let url = engine.suggestion_url.replace("%s", &search_term);
                let tx2 = tx.clone();
                let sugg_thread = thread::spawn(move || {
                    tx2.send(UiMsg::SetSuggestions(fetch_suggs(url).ok()));
                });
            }
            match stdin.next() {
                Some(ie) => match ie {
                    InputEvent::Keyboard(k) => match k {
                        KeyEvent::Char('\n') => {
                            input_buf.clear();
                            tx.send(UiMsg::Finish(format!(
                                "Searching for {} on {}!",
                                search_term, engine.name
                            )));
                            break;
                        }
                        KeyEvent::Char('\t') | KeyEvent::Ctrl('n') | KeyEvent::Down => {
                            tx.send(UiMsg::SelNext);
                        }
                        KeyEvent::BackTab | KeyEvent::Ctrl('p') | KeyEvent::Up => {
                            tx.send(UiMsg::SelPrev);
                        }
                        KeyEvent::Char(character) => {
                            input_buf.push(character as char);
                        }

                        KeyEvent::Backspace => {
                            input_buf.pop();
                        }
                        KeyEvent::Ctrl(c) => match c {
                            'c' => {
                                tx.send(UiMsg::Quit);
                                break;
                            }
                            'w' => {
                                // delete last word (trim, then delete backwards until first
                                // word character or beginning of line
                                input_buf = input_buf
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
            };
        }
    });
    let ui_thread = thread::spawn(move || {
        ui_loop(rx, terminal, cursor);
    });
    input_thread.join();
    ui_thread.join();
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
struct Engine {
    prompt: Prompt,
    name: String,
    suggestion_url: String,
    search_url: String,
}

fn match_engine<'a, 'b>(
    input_line: &'b str,
    engines: &'a HashMap<String, Engine>,
) -> (&'a Engine, String) {
    let default_engine = engines.get("").unwrap();
    let words: Vec<&str> = input_line.split_whitespace().collect();
    if words.len() < 1 || (words.len() == 1 && !input_line.ends_with(" ")) {
        return (default_engine, input_line.trim().to_string());
    }
    let potential_prefix = words.first().unwrap();
    match engines.get(&potential_prefix.to_string()) {
        Some(engine) => {
            let search_term = input_line[potential_prefix.len()..].trim().to_string();
            (engine, search_term)
        }
        None => (default_engine, input_line.trim().to_string()),
    }
}

fn define_engines() -> HashMap<String, Engine> {
    let mut engs = HashMap::new();
    engs.insert(
        "".to_string(),
        Engine {
            name: "Google".to_string(),
            suggestion_url: "https://www.google.com/complete/search?client=chrome&q=%s".to_string(),
            search_url: "null".to_string(),
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
        "wkt".to_string(),
        Engine {
            name: "Wiktionary".to_string(),
            suggestion_url: "https://en.wiktionary.org/w/api.php?action=opensearch&search=%s&limit=10&namespace=0&format=json".to_string(),
            search_url: "null".to_string(),
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
            search_url: "null".to_string(),
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
            search_url: "null".to_string(),
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
    Quit,
    Finish(String),
    SetInputLine(Prompt, String),
    SetSuggestions(Option<Suggestions>),
    ExpectSuggsFor(String),
    SelNext,
    SelPrev,
    Nop,
}

#[allow(unused_must_use)]
fn ui_loop(rx: Receiver<UiMsg>, terminal: Terminal, mut cursor: TerminalCursor) {
    cursor.hide();
    let mut input_line: String = String::from("");
    let mut suggs: Option<Suggestions> = None;
    let mut prompt: Option<Prompt> = None;
    let mut waiting_for_term: Option<String> = None; //
    let mut counter = 0;
    let (t_w, t_h) = terminal.terminal_size();
    let mut selected_n: usize = 0;
    loop {
        // draw ui line
        let mut lines_printed = 0;
        cursor.move_left(t_w);
        terminal.clear(ClearType::CurrentLine);
        match &prompt {
            Some(promp) => {
                print!(
                    "{}{}{}{}",
                    Colored::Fg(promp.icon_fg),
                    Colored::Bg(promp.icon_bg),
                    promp.icon,
                    Attribute::Reset
                ); // icon
                print!(
                    "{}{}{}{}",
                    Colored::Fg(promp.text_fg),
                    Colored::Bg(promp.text_bg),
                    promp.text,
                    Attribute::Reset
                ); // prompt text
            }
            None => {}
        };
        println!(" {}_", input_line);
        lines_printed += 1;
        let suggest_lines = 10; /*if let Some(ref suggs) = suggs {
            suggs.sugg_terms.len()
        } else {
            0
        }; */
        if let Some(ref suggs) = suggs {
            for (n, line) in suggs.sugg_terms.iter().enumerate() {
                terminal.clear(ClearType::CurrentLine);
                cursor.move_left(t_w);
                if selected_n == n {
                    println!("{}", line);
                } else {
                    println!("{}", line);
                }
                lines_printed += 1;
            }
        }
        let msg = rx.recv().unwrap();
        match msg {
            UiMsg::Quit => {
                terminal.clear(ClearType::CurrentLine);
                for _ in 0..suggest_lines {
                    cursor.move_up(1);
                    terminal.clear(ClearType::CurrentLine);
                }
                break;
            }
            UiMsg::Finish(s) => {
                for _ in 0..suggest_lines {
                    cursor.move_up(1);
                    terminal.clear(ClearType::CurrentLine);
                }
                cursor.move_left(t_w);
                println!("{}", s);
                terminal.clear(ClearType::CurrentLine);
                break;
            }
            UiMsg::SetInputLine(new_prompt, s) => {
                input_line = s;
                prompt = Some(new_prompt);
            }
            UiMsg::SetSuggestions(new_sugg) => {
                match new_sugg {
                    Some(ref new_sugg) => match waiting_for_term {
                        Some(ref expected_term) if expected_term == &new_sugg.term => {
                            if let Some(ref old_suggs) = suggs {
                                let n_delta = old_suggs.sugg_terms.len() as i32
                                    - new_sugg.sugg_terms.len() as i32;
                                if n_delta > 0 {
                                    for _ in 0..n_delta as usize {
                                        cursor.move_up(1);
                                        terminal.clear(ClearType::CurrentLine);
                                        lines_printed -= 1;
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    None => {
                        if let Some(ref suggs) = suggs {
                            for _ in 0..suggs.sugg_terms.len() as usize {
                                cursor.move_up(1);
                                terminal.clear(ClearType::CurrentLine);
                                lines_printed -= 1;
                            }
                        }
                    }
                }
                suggs = new_sugg
            }
            UiMsg::Nop => {}
            UiMsg::SelNext => {
                selected_n += 1;
                if selected_n >= suggest_lines {
                    selected_n = 0;
                }
            }
            UiMsg::SelPrev => {
                selected_n = selected_n
                    .checked_sub(1)
                    .unwrap_or(max(suggest_lines - 1, 0));
            }
            UiMsg::ExpectSuggsFor(term) => {
                waiting_for_term = Some(term);
            }
        };
        counter += 1;
        cursor.move_up(lines_printed);
    }
    cursor.show();
    cursor.move_left(t_w);
}
