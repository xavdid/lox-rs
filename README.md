# lox-rs

A Rust implementation the first Lox interpreter from [Crafting Interpreters](https://craftinginterpreters.com/). Written as part of Dave Beazley's [Crusty Interpreters](https://www.dabeaz.com/crusty.html) course.

## Potential Improvements

- [ ] repl should show expression results
- [x] error handling could be improved, especially inside the parser and interpreter
  - I don't really need all those enum types, something that wraps a string would be fine
  - Should probably try out `thiserror` for practice
- [ ] similarly, the interpreter can't point to specific bits of code that generated errors; should store the tokens (which at least have a line number) to give some pointers
  - [ ] try `miette` maybe?
- [ ] pulling values from the env clones them, which isn't great
- [ ] the parser's helper functions could use renaming / cleanup
