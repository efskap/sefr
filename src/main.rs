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

extern crate crossterm;
extern crate directories;
extern crate json;
extern crate minreq;
extern crate serde;
extern crate toml;
extern crate webbrowser;

use crossterm::{
    input, Attribute, ClearType, Color, Colored, Crossterm, InputEvent, KeyEvent, RawScreen,
};

use std::cmp::min;
use std::collections::HashMap;
use std::result::Result;
use std::sync::mpsc;
use std::thread;

mod config;
mod engine;
mod suggestion_adapter;
mod util;

use config::*;
use engine::*;
use suggestion_adapter::*;
use util::*;

#[allow(unused_must_use)]
fn main() {
    let (tx, rx) = mpsc::channel();

    let config = get_config();
    let engines = config.engines;
    let _screen = RawScreen::into_raw_mode();
    let crossterm = Crossterm::new();
    let mut cursor = crossterm.cursor();
    let terminal = crossterm.terminal();

    let input = input();
    let mut stdin = input.read_sync();
    let mut input_buf = String::new();

    let keybindings = config.keybinds;
    let input_tx = tx.clone();
    let input_thread = thread::spawn(move || loop {
        let key = stdin.next();
        match key {
            Some(InputEvent::Keyboard(k)) => {
                let keybind = KeyBind { 0: k };
                match keybindings.get(&keybind) {
                    Some(BindableAction::Submit) => {
                        input_tx.send(UiMsg::OnInput(BindableAction::Submit));
                        break;
                    }
                    Some(BindableAction::Exit) => {
                        input_tx.send(UiMsg::OnInput(BindableAction::Exit));
                        break;
                    }
                    Some(act) => {
                        input_tx.send(UiMsg::OnInput(act.clone()));
                    }
                    None => {
                        if let KeyEvent::Char(c) = keybind.0 {
                            input_tx.send(UiMsg::OnInput(BindableAction::AddChar(c)));
                        }
                    }
                }
            }
            _ => {}
        }
    });

    cursor.hide();
    let mut input_line: String = String::from("");
    let mut suggs: Option<Suggestions> = None;
    let mut prompt = &engines.get("").expect("No default engine set.").prompt;
    let mut waiting_for_term: Option<String> = None; // the term for which we are expecting suggestions (in case of out-of-order resolves)
    let mut selected_n: Option<usize> = None;

    let mut prev_engine: Option<&Engine> = None;
    let mut refresh_completions = true;

    let mut t_w: u16;
    // main UI loop
    loop {
        t_w = terminal.terminal_size().0; // refresh terminal width in case it was resized
        let (engine, prefix, search_term) = match_engine(&input_line, &engines);
        if let Some(ref prev_engine) = prev_engine {
            // if the engine has changed (based on suggestion url)
            if prev_engine.suggestion_url != engine.suggestion_url {
                suggs = None; // clear the list that gets drawn asap
                refresh_completions = true; // and force an update
            }
        }
        prev_engine = Some(&engine);
        if refresh_completions {
            prompt = &engine.prompt;
            if search_term.is_empty() {
                suggs = None;
                waiting_for_term = None;
            } else {
                waiting_for_term = Some(search_term.clone());
                let url = engine.format_suggestion_url(&search_term);
                let tx2 = tx.clone();
                // TODO: this should be done with boxed traits I think?
                let sugg_adapter = engine.suggestion_adapter.clone();
                let search_term2 = search_term.clone();
                // spawn a separate thread to do the http request and send the result to the
                // channel that this thread is receiving on
                if !url.is_empty() {
                    thread::spawn(move || {
                        let resolved_suggs = match sugg_adapter {
                            SuggestionAdapterName::OpenSearch => {
                                OpenSearchAdapter::get(url, search_term2)
                            }
                            SuggestionAdapterName::JsonPath(path) => {
                                JsonPathAdapter(path).get(url, search_term2)
                            }
                        };
                        match resolved_suggs{
                            Ok(resolved_suggs) => {
                                tx2.send(UiMsg::SetSuggestions(resolved_suggs));
                            }
                            Err(e) => {
                                // hacky but i want the error to be seen
                                for _ in 1..20 {
                                    println!("Error: {}", e);
                                }
                            }
                        }
                    });
                }
            }
            refresh_completions = false;
            selected_n = None;
        }
        cursor.move_left(t_w);
        terminal.clear(ClearType::CurrentLine);

        let full_prompt_line = format!("{} {}_", prompt, input_line);
        if full_prompt_line.len() >= t_w as usize {
            let short_prompt = format!("{}", prompt.to_short());
            print!("{}", short_prompt);
            // 2 = spacer + cursor
            // TODO: don't count control characters for length
            let room_for_input_line = (t_w as usize)
                .checked_sub(short_prompt.len() + 2)
                .unwrap_or(0);
            let truncated_input_line = truncate_from_end(&input_line, room_for_input_line);
            println!(" {}_", truncated_input_line);
        } else {
            // just printing full_prompt_line doesn't preserve colours for some reason
            println!("{} {}_", prompt, input_line);
        }

        let suggest_lines = 15; /*if let Some(ref suggs) = suggs {
                                    suggs.sugg_terms.len()
                                } else {
                                    0
                                }; */
        let selectable_lines = if let Some(ref suggs) = suggs {
            min(suggest_lines, suggs.sugg_terms.len())
        } else {
            0
        };
        for n in 0..suggest_lines {
            terminal.clear(ClearType::CurrentLine);
            cursor.move_left(t_w);

            if let Some(ref suggs) = suggs {
                match suggs.sugg_terms.get(n) {
                    Some(line) => {
                        let line_trunc = truncate_from_end(&line, t_w as usize);
                        match selected_n {
                            Some(selected_n) if selected_n == n => {
                                print!(
                                    "{}{}{}{}",
                                    Colored::Fg(Color::Black),
                                    Colored::Bg(Color::White),
                                    line_trunc,
                                    Attribute::Reset
                                );
                            }
                            _ => {
                                print!("{}", line_trunc);
                            }
                        }
                    }
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
                    BindableAction::Submit => {
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
                    BindableAction::SelectNext => {
                        if let Some(ref suggs) = suggs {
                            selected_n = Some(if let Some(selected_n) = selected_n {
                                selected_n + 1
                            } else {
                                0
                            });
                            if selected_n.unwrap() >= selectable_lines {
                                selected_n = Some(0);
                            }
                            if let Some(selected) = suggs.sugg_terms.get(selected_n.unwrap()) {
                                let (_, interfering_prefix, _) = match_engine(&selected, &engines);
                                input_line = input_line_from_selection(
                                    &interfering_prefix,
                                    &prefix,
                                    &selected,
                                );
                                refresh_completions = false;
                            }
                        }
                    }
                    BindableAction::SelectPrev => {
                        if let Some(ref suggs) = suggs {
                            selected_n = Some(
                                selected_n
                                    .unwrap_or(0)
                                    .checked_sub(1)
                                    .unwrap_or(selectable_lines.checked_sub(1).unwrap_or(0)),
                            );
                            if let Some(selected) = suggs.sugg_terms.get(selected_n.unwrap()) {
                                let (_, interfering_prefix, _) = match_engine(&selected, &engines);
                                input_line = input_line_from_selection(
                                    &interfering_prefix,
                                    &prefix,
                                    &selected,
                                );
                                refresh_completions = false;
                            }
                        }
                    }

                    BindableAction::DeleteChar => {
                        input_line.pop();
                        selected_n = None;
                    }
                    BindableAction::Exit => {
                        terminal.clear(ClearType::CurrentLine);
                        for _ in 0..suggest_lines {
                            cursor.move_up(1);
                            terminal.clear(ClearType::CurrentLine);
                        }
                        break;
                    }
                    BindableAction::DeleteWord => {
                        // delete last word (trim, then delete backwards until first
                        // word character or beginning of line
                        input_line = input_line
                            .trim_end()
                            .trim_end_matches(|x: char| !x.is_whitespace())
                            .to_string();
                    }
                    BindableAction::AddChar(character) => {
                        // if we don't want spaces in the search term, like with subreddits
                        // just make space only select a suggestion if one is highlighted
                        if !(character == ' ' && engine.space_becomes.is_empty()) {
                            input_line.push(character as char);
                        }
                        selected_n = None;
                    }
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

fn input_line_from_selection(
    prefix_in_result: &str,
    current_prefix: &str,
    selected_result: &str,
) -> String {
    if !prefix_in_result.is_empty() {
        // selected_result starts with a prefix that matches one of our engines
        if !current_prefix.is_empty() {
            // but we already have a prefix entered, so no escaping needed
            // just use the result while keeping our existing prefix
            format!("{} {}", current_prefix, selected_result)
        } else {
            // don't want the prefix in the result to trigger an engine, so escape it
            format!("?{}", selected_result)
        }
    } else {
        // no conflict, so just use the result while keeping our existing prefix
        if !current_prefix.is_empty() {
            format!("{} {}", current_prefix, selected_result)
        } else {
            // our prefix is empty, and the result doesn't start with a prefix
            // this is a separate branch so that a space isn't inserted before the result
            format!("{}", selected_result)
        }
    }
}

// mirrors opensearch schema
#[derive(Debug)]
pub struct Suggestions {
    term: String,
    sugg_terms: Vec<String>,
    //descriptions: Vec<String>,
    //urls: Vec<String>
}

enum UiMsg {
    SetSuggestions(Suggestions),
    OnInput(BindableAction),
}
