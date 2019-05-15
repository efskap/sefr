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

## progress

Currently messy but it's working.

- [x] Prompt (albeit with fake cursor)
- [x] Suggestions request / json parse
- [x] Definable engines with prefixes, prompts, and endpoints
- [x] Browser launching
- [x] Selection of suggestions w/ prefix edge cases
- [x] TOML file config
