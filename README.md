# lox-rs

A Rust implementation the first Lox interpreter from [Crafting Interpreters](https://craftinginterpreters.com/). Written as part of Dave Beazley's [Crusty Interpreters](https://www.dabeaz.com/crusty.html) course.

I'm only passable at Rust, so I'm sure there's lots of room for improvement.

## Current Status

As of 2026-06-06, I've finished Chapter 10 of the book (through function calls). Everything seems to work, if inefficiently. I think I'm going to set this down for now in favor of other work, but it's been a fun project!

I may come back to tinker eventually, but I think I've gotten what I wanted (experience with larger rust codebase, common 3rd party crates, etc) out of it.

## Potential Improvements

- [x] repl should show expression results
- [x] error handling could be improved, especially inside the parser and interpreter
  - I don't really need all those enum types, something that wraps a string would be fine
  - Should probably try out `thiserror` for practice
- [ ] similarly, the interpreter can't point to specific bits of code that generated errors; should store the tokens (which at least have a line number) to give some pointers
  - [ ] try `miette` maybe?
- [ ] pulling values from the env clones them, which isn't great
- [x] the parser's helper functions could use renaming / cleanup
