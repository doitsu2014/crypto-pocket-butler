# Command by BASH
# export DATABASE_URL="postgres://postgres:postgres@localhost/crypto_pocket_butler" && cargo run -- up

# PS1

$env:DATABASE_URL = "postgres://postgres:postgres@localhost/crypto_pocket_butler"
cargo run -- up