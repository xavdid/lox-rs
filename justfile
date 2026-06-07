
set quiet
set no-exit-message

_default:
  just --list

run:
  cargo run --quiet
run-release:
  cargo run --release --quiet

lint *args:
  cargo clippy --all-targets {{ args }}

COURSE_DIR := "/Users/david/projects/crusty/lox-rs"

sync message:
  cp -r src examples cargo.toml cargo.lock {{ COURSE_DIR }}
  git -C {{ COURSE_DIR }} add .
  git -C {{ COURSE_DIR }} commit -m "{{ message }}"

# make the commit in both branches
sync-all message: && (sync message)
  gg "{{ message }}"

alias s := sync
alias sa := sync-all

example name:
  cargo run --quiet examples/{{ name }}.lox
alias ex := example
