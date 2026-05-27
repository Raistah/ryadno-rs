# Complex example of how Ryadno forms works

## Installation
```bash
cargo install && \
npm install
```

## Running
```bash
    cargo run -p ryadno --bin make -- -f publish-templates && \
    npx @tailwindcss/cli -i ./assets/style.css -o ./public/style.css && \
    cargo run
```

### Running with watch
```bash
cargo watch -i "public/**/*" -i "templates/ryadno/**/*" -s "cargo run -p ryadno --bin make -- -f p
ublish-templates && npx @tailwindcss/cli -i ./assets/style.css -o ./public/style.css && cargo run"
```
