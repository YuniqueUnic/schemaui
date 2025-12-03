# Configure PowerShell for Windows
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

# show the recipe list
default:
    @just --list

# install all needed tools (Unix: bash, macOS, Linux)
[unix]
init:
    rustup component add rust-analyzer clippy rustfmt
    cargo binstall prek --git https://github.com/j178/prek 2>/dev/null || cargo install --locked --git https://github.com/j178/prek
    cd web/ui && pnpm install

# install all needed tools (Windows: PowerShell)
[windows]
init:
    rustup component add rust-analyzer clippy rustfmt
    cargo binstall prek --git https://github.com/j178/prek 2>$null; if ($LASTEXITCODE -ne 0) { cargo install --locked --git https://github.com/j178/prek }
    cd web/ui; pnpm install

# install prek (which is the alternative tool of pre-commit)
install-prek:
    prek uninstall
    prek install .

# test schemaui related things
test:
    cargo test --workspace -F full

# build the web ui into web/dist (Unix: bash, macOS, Linux)
[unix]
build-web:
    rm -rf web/dist
    cd web/ui && pnpm build:embedded

# build the web ui into web/dist (Windows: PowerShell)
[windows]
build-web:
    if (Test-Path web/dist) { Remove-Item -Recurse -Force web/dist }
    cd web/ui; pnpm build:embedded

# build the cli
build-cli:
    cargo build -p schemaui-cli -F full

# build everything(cli, web)
build: build-web
    @just build-cli

# run prek
prek +ARGS="-a":
    prek run {{ARGS}}

# run clippy and rustfmt, then run prek
happy:
    cargo clippy --fix --allow-dirty --tests -- -D warnings
    cargo fmt --all
    just prek

# run dev web server (Unix: bash, macOS, Linux)
[unix]
dev-web port="5173":
    lsof -ti:{{port}} 2>/dev/null | xargs kill -9 2>/dev/null || true
    cd web/ui && pnpm dev --host 0.0.0.0 --port {{port}}

# run dev web server (Windows: PowerShell)
[windows]
dev-web port="5173":
    Get-Process -Id (Get-NetTCPConnection -LocalPort {{port}} -ErrorAction SilentlyContinue).OwningProcess -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    cd web/ui; pnpm dev --host 0.0.0.0 --port {{port}}


alias pre-commit := prek
alias lint := happy
alias b := build
alias t := test
alias cli := build-cli
alias web := build-web
alias dev := dev-web
