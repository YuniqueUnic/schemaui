
# show the recipe list
default:
    @just --list

# install all needed tools
init:
    rustup component add rust-analyzer clippy rustfmt
    cargo binstall prek --git https://github.com/j178/prek # need to check the bisntall command
    # cargo install --locked --git https://github.com/j178/prek # need to check the install command
    cd web/ui && pnpm install  # if no pnpm, use npm: `npm install`

# install prek (which is the alternative tool of pre-commit)
install-prek:
    prek uninstall
    prek install .

# test schemaui related things
test:
    cargo test --workspace -F full

# build the web ui into web/dist
build-web:
    rm -rf web/dist
    cd web/ui && pnpm build

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
    cargo clippy --fix --allow-dirty -- -D warnings
    cargo fmt --all
    just prek

# run dev web
dev-web port="5173":
    lsof -ti:{{port}} 2>/dev/null | xargs kill -9 2>/dev/null || true # kill old process
    cd web/ui && pnpm dev --host 0.0.0.0 --port {{port}}


alias pre-commit := prek
alias lint := happy
alias b := build
alias t := test
alias cli := build-cli
alias web := build-web
alias dev := dev-web
