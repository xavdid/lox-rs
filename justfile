
set quiet

_default:
  just --list

run:
  cargo run --quiet
run-release:
  cargo run --release --quiet

lint *args:
  cargo clippy --all-targets {{ args }}

COURSE_DIR:= "/Users/david/projects/crusty/lox-rs"

sync message:
  cp -r src cargo.toml cargo.lock {{ COURSE_DIR }}
  git -C {{ COURSE_DIR }} add .
  git -C {{ COURSE_DIR }} commit -m "{{ message }}"
  git -C {{ COURSE_DIR }} push

sync-all message: && (sync message)
  gg "{{ message }}"

alias s := sync
alias sa := sync-all
