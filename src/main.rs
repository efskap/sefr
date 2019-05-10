extern crate crossterm;
extern crate tui;

use crossterm::{
    cursor, input, terminal, AlternateScreen, Attribute, ClearType, Color, Colored, Crossterm,
    InputEvent, KeyEvent, RawScreen, Terminal, TerminalCursor,
};

use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::result::Result;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{process, thread, time};

const SUGGEST_LINES: u16 = 10;

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
            tx.send(UiMsg::SetInputLine(engine.prompt.clone(), input_buf.clone()));
            match stdin.next() {
                Some(ie) => match ie {
                    InputEvent::Keyboard(k) => match k {
                        KeyEvent::Char('\n') => {
                            input_buf.clear();
                            tx.send(UiMsg::Finish( format!("Searching for {} on {}!", search_term, engine.name)));
                            break;
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
                            },
                            'w' => {
                                // delete last word (trim, then delete backwards until first
                                // word character or beginning of line
                                input_buf = input_buf.trim_end().trim_end_matches(|x: char|!x.is_whitespace()).to_string();

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

struct Engine {
    prompt: Prompt,
    name: String,
    suggestion_url: String,
    search_url: String,
}

fn match_engine<'a, 'b>(input_line: &'b str, engines: &'a HashMap<String, Engine>) -> (&'a Engine, String) {
    let default_engine = engines.get("").unwrap();
    let words: Vec<&str> = input_line.split_whitespace().collect();
    if words.len() < 1 || (words.len() == 1 && !input_line.ends_with(" ")){
        return (default_engine, input_line.trim().to_string());
    }
    let prefix = words.first().unwrap();
    let engine = engines.get(&prefix.to_string()).unwrap_or(default_engine);
    let search_term = input_line[prefix.len()..].trim().to_string();
    (engine, search_term)
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
            suggestion_url: "null".to_string(),
            search_url: "null".to_string(),
            prompt: Prompt {
                icon_fg: Color::Black,
                icon_bg: Color::White,
                icon: String::from(" W "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: String::from(" Wiktionary "),
            },
        },
    );
    engs.insert(
        "yt".to_string(),
        Engine {
            name: "YouTube".to_string(),
            suggestion_url: "http://suggestqueries.google.com/complete/search?client=firefox&ds=yt&q=%s".to_string(),
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
            suggestion_url: "null".to_string(),
            search_url: "null".to_string(),
            prompt:  Prompt {
                icon_fg: Color::White,
                icon_bg: Color::Red,
                icon: String::from(" ⭖ "),
                text_fg: Color::Black,
                text_bg: Color::White,
                text: String::from(" Subreddit ")
            }
        },
    );
    engs
}
// mirrors opensearch schema
struct Suggestions {
    term: String,
    sugg_terms: Vec<String>,
    descriptions: Vec<String>,
    urls: Vec<String>,
    n_selected: i32,
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

enum UiMsg{
    Quit,
    Finish(String),
    SetInputLine( Prompt, String),
    SetSuggestions(Suggestions),
    Nop,
}

fn ui_loop(rx: Receiver<UiMsg>, terminal: Terminal, mut cursor: TerminalCursor) {
    cursor.hide();
    let mut input_line: String = String::from("");
    let mut suggs = Suggestions {
        term: String::from(""),
        sugg_terms: vec![],
        descriptions: vec![],
        urls: vec![],
        n_selected: 0,
    };
    let default_prompt = Prompt {
        icon_fg: Color::White,
        icon_bg: Color::Red,
        icon: String::from(" ▶ "),
        text_fg: Color::Black,
        text_bg: Color::White,
        text: String::from(" YouTube "),
    };
    let mut prompt: Option<Prompt>  = None;
    let mut counter = 0;
    let (t_w, t_h) = terminal.terminal_size();
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
            None  => {}
        };
        println!(" {}_", input_line);
        lines_printed += 1;
        for (n, line) in suggs.sugg_terms.iter().enumerate() {
            terminal.clear(ClearType::CurrentLine);
            cursor.move_left(t_w);
            println!("{}", line);
            lines_printed += 1;
        }
        for i in 1..SUGGEST_LINES {
            terminal.clear(ClearType::CurrentLine);
            cursor.move_left(t_w);
            println!("{} {}", counter + i, input_line);
            lines_printed += 1;
        }
        let msg = rx.recv().unwrap();
        let n_clear = suggs.sugg_terms.len();
        match msg {
            UiMsg::Quit => {
                terminal.clear(ClearType::CurrentLine);
                for i in 1..SUGGEST_LINES {
                    cursor.move_up(1);
                    terminal.clear(ClearType::CurrentLine);
                }
                break;
            },
            UiMsg::Finish(s) => {
                for i in 1..SUGGEST_LINES {
                    cursor.move_up(1);
                    terminal.clear(ClearType::CurrentLine);
                }
                cursor.move_left(t_w);
                println!("{}", s);
                terminal.clear(ClearType::CurrentLine);
                break;
            },
            UiMsg::SetInputLine(new_prompt, s) => {
                input_line = s;
                prompt = Some(new_prompt);
            }
            UiMsg::SetSuggestions(sugg) => {
                suggs = sugg;
            }
            Nop => {}
        };
        counter += 1;
        cursor.move_up(lines_printed);
    }
    cursor.show();
    cursor.move_left(t_w);
}
