# sefr (Search Engine FRontend)

Terminal program for interactively opening search engines / parametric URLs.  
It's kinda like surfraw but with interactive suggestions (via parsing opensearch json).

![](https://github.com/efskap/sefr/raw/master/demo.gif "demo gif")

## motivation

I use custom url bar search engines a lot, but browser support for them is frustrating.

 - Firefox has a really obtuse way of defining them, doesn't let you specify suggestion endpoints, and still doesn't sync them.
 - Chrome makes defining them easy, syncs them, but doesn't let you specify suggestion endpoints.
 - Vivaldi makes defining them easy, lets you specify suggestion endpoints, but doesn't sync them.

e.g. in stock firefox, you can't create a search engine that, when you type "r foo" in your url bar, automatically goes to "reddit.com/r/foo".
You have to manually write the URL, and you don't even get completions!

This is meant to be a customizable crossplat solution, and since it uses your default browser ([details](https://github.com/amodm/webbrowser-rs#examples)), I hope to fit it into my workflow with a global keybinding.

## running

Clone it, install [the Rust toolchain](https://rustup.rs/), and run `cargo run` in the directory.

You can also install it with `cargo install sefr`.

Keybindings are hardcoded right now, and they are:

- Tab/Down/Ctrl-N => Select next suggestion
- Shift-Tab/Up/Ctrl-P => Select previous suggestion
- Ctrl-W => Delete last word
- Ctrl-C => Exit
- Enter => Perform selected search in browser

## config

Generates a TOML file in the the config dir provided by the [directories crate](https://crates.io/crates/directories) (the usual ones, e.g. ~/.config/sefr/config.toml on linux). Should be pretty straightforward to add new search engines but sorry if I break the format between development versions.

You can leave out the prompt section when adding new engines and it'll use the default one. Also if you leave out the prompt text (or the entire prompt section) it'll display the engine's "name" parameter there.

So in that sense, the only _required_ parameters for a new engine are:

- `name`
- `search_url` (opened in browser with `%s` replaced by the search term upon hitting enter)
- `suggestion_url` (endpoint returning [OpenSearch suggestions schema json](http://www.opensearch.org/Specifications/OpenSearch/Extensions/Suggestions) with `%s` replaced by the search term, queried for suggestions while typing)

The engine used when no prefix is entered is defined as `_default` in the config, and it is obligatory for the program to start.

So, minimal config.toml file:

```toml
[engines._default]
name = "Google"
search_url = "https://www.google.com/search?q=%s"
suggestion_url = "https://www.google.com/complete/search?client=chrome&q=%s"
```

Fields are all strings except for colors (`*_bg`, `*_fg`). They can be strings (corresponding to [the color names here](https://github.com/TimonPost/crossterm/blob/master/crossterm_style/src/enums/color.rs)), 8-bit numbers (corresponding to [Ansi color codes](https://jonasjacek.github.io/colors/)), or 8-bit RGB tuples like `[255,255,255]`

The default config it generates is much less minimal, demonstrating prompt customization.

## progress

Currently messy but it's working.

- [x] Prompt
- [x] Suggestions request / json parse
- [x] Definable engines with prefixes, prompts, and endpoints
- [x] Browser launching
- [x] Selection of suggestions w/ prefix edge cases
- [x] TOML file config
- [ ] Use real cursor for rendering input buffer, and be able to move it
- [ ] Configurable keybindings
- [ ] Better feedback for when suggestion endpoints misbehave
- [ ] CLI args, e.g. providing the initial input buffer through an argument for aliasing.
