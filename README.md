# sefr (Search Engine FRontend)

[![Crates.io](https://img.shields.io/crates/v/sefr.svg) ![Crates.io](https://img.shields.io/crates/d/sefr.svg)](https://crates.io/crates/sefr)

Terminal program for interactively opening search engines / parametric URLs.  
It's kinda like surfraw but with interactive suggestions (via parsing opensearch json).

![](https://github.com/efskap/sefr/raw/master/demo.gif "demo gif")

## Motivation

I use custom url bar search engines a lot, but browser support for them is frustrating.

 - Firefox has a really obtuse way of defining them, doesn't let you specify suggestion endpoints, and still doesn't sync them.
 - Chrome makes defining them easy, syncs them, but doesn't let you specify suggestion endpoints.
 - Vivaldi makes defining them easy, lets you specify suggestion endpoints, but doesn't sync them.

e.g. in stock Firefox, you can't create a search engine that, when you type "r foo" in your url bar, automatically goes to "reddit.com/r/foo".
You have to manually write the URL, and you don't even get completions!

This is meant to be a customizable crossplatform solution, and since it uses your default browser ([more details](https://github.com/amodm/webbrowser-rs#examples)), you can integrate it into a GUI workflow with a global hotkey (see below).

## Installation

There are two ways to install `sefr`:
1. Clone this repository, install [the Rust toolchain](https://rustup.rs/), and either call `cargo run` in the cloned directory to try it out, or `cargo build` to create a binary located at `target/debug/sefr`.
2. Install via cargo by calling `cargo install sefr`. This should make it runnable from anywhere.

A convenient way to integrate it into your desktop environment is by mapping a global hotkey to launch it in a lightweight terminal. For example, bind this to a convenient hotkey in your DE or WM and change the last argument to point at the binary.

```sh
uxterm  -geometry 60x20+620+200 -fa 'Monospace' -fs 13 -bg grey27 -fg white -e ~/sefr/target/debug/sefr
```

For i3 + suckless terminal, this works well. Not sure why I have to specify `$BROWSER`.

```sh
for_window [instance="^sefr$"] floating enable, resize set 640 480, move position center
bindsym Mod4+s exec BROWSER=/usr/bin/firefox st -n sefr -f 'Monospace:size=14' -e ~/sefr/target/debug/sefr
```

## Configuration  / Customization

### Config file
On its first startup, `sefr` will automatically generate a TOML configuration file in the config directory provided by the [directories crate](https://crates.io/crates/directories). Any subsequent changes should be made inside it.

e.g. For Linux, the config file will be found in `~/.config/sefr/config.toml`.

### Adding new engines
__Warning: The current configuration format might be changed in the future!__

New engines can be added for use by `sefr` by adding them to the `config.toml` file. 

A basic engine definition looks like this:

```toml
[engines.yt]
name = "YouTube"
search_url = "https://www.youtube.com/results?q=%s"
suggestion_url = "http://suggestqueries.google.com/complete/search?client=firefox&ds=yt&q=%s"
```

- `[engines.PREFIX]` defines what _prefix_ (also known as a _keyword_ or _trigger_) activates the engine.
- `name` is the name of the engine, used for the prompt text if not defined in the prompt section (more on that later).
- `search_url` is opened in your browser with `%s` replaced by the search term when enter is pressed.
- `suggestion_url` (optional) is the endpoint queried for suggestions (with `%s` replaced by the search term) while typing. It must return  [OpenSearch suggestions schema json](http://www.opensearch.org/Specifications/OpenSearch/Extensions/Suggestions).
- `space_becomes` (optional, `+` by default) is what spaces are replaced with before `search_url` is opened.  
In the default config:
  - `engines.wkt` (Wiktionary) and `engines.w` (Wikipedia) have it set to `_`, because that's how Wikis encode spaces in their URLs.
  - `engines.r` (Subreddit) has it set to a blank string, because subreddits can't have spaces in their names (note that this value prevents spaces from being entered into the input buffer when the engine is selected so that space can be used to select a suggestion without performing a search).

The engine used when no prefix is entered is defined as `_default` in the config, and it is obligatory for the program to start. Example:

```toml
[engines._default]
name = "Google"
search_url = "https://www.google.com/search?q=%s"
suggestion_url = "https://www.google.com/complete/search?client=chrome&q=%s"
```

Along with this, there is also an optional `prompt` section which handles the prompt displayed when the engine is called. It will usually look like this:

```toml
[engines.yt.prompt]
icon = " ▶ "
icon_bg = "Red"
icon_fg = "White"
text = " Youtube "
text_bg = "White"
text_fg = "Black"
```

The following fields are supported, and all are optional:
- `icon`: the icon displayed in the prompt
- `icon_bg`: background color of the icon
- `icon_fg`: foreground color of the icon
- `text`: The text displayed after the icon in the prompt
- `text_bg`: background color for the text
- `text_fg`: foreground color for the text

Note that `icon` and `text` are padded with whitespace for aesthetics in the example configuration, but this is not required.

The fields are all strings except for colors (`*_bg`, `*_fg`). They can be strings (corresponding to [the color names here](https://github.com/TimonPost/crossterm/blob/master/crossterm_style/src/enums/color.rs)), 8-bit numbers (corresponding to [Ansi color codes](https://jonasjacek.github.io/colors/)), or 8-bit RGB tuples like `[255,255,255]`

If this section is left out for a particular engine, a basic prompt displaying the engine's name will be used.

### Keybindings

Keybindings are a work in progress, but all of the current functions are rebindable under the `[keybinds]` section.   
Keybinds are in a vim-like syntax (e.g. `<Down>`, `<C-w>`, `<F12>`), but there are a few things to note:

- The binding and action are double quoted. So the entire binding is a line like `"<Backspace>" = "DeleteChar"`.

- All bindings are in <angle brackets> except single characters (e.g. the literal letter `p`, as in `"p" = "Exit"`).    
 But why would you make a binding like that?

- `Ctrl` is represented by `C-` (e.g. `<C-w>` means 'control + w')

- Everything inside `<`angle brackets`>` is case-insensitive except the normal key after a modifier.    
That is, `<a-p>`, `<A-p>`, `<m-p>`, and `<M-p>` all mean the same thing ('alt + p') but `<m-P>` means 'alt + shift + p'.

- So that means there is no "shift" modifier. To register 'alt + shift + s' you'd write `<a-S>` or `<A-S>`.

- 'shift + tab' is represented by `<Backtab>`.

- 'enter' can be `<CR>` or `<Enter>`. 'backspace' can be `<BS>` or `<Backspace>`.

- An empty string represents the `NULL` character, whatever that is.

- If you assign two functions two the same key, the one registered later will override the first.

Excerpt from the default config:

```toml
[keybinds]
"<BackTab>" = "SelectPrev"
"<Backspace>" = "DeleteChar"
"<C-c>" = "Exit"
```

## Progress

This project is currently in its **alpha** stage but is relatively stable.

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
