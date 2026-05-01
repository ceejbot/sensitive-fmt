_help:
    just -l

# Run all tests using nextest.
test:
    cargo nextest run

coverage:
    cargo llvm-cov --all-targets --workspace --summary-only

# Run the nightly formatter
@fmt:
    cargo +nightly fmt

# Run a security audit
@audit:
    cargo audit

# Run a format check.
@lint:
    cargo +nightly fmt --check

# Run the same checks we run in CI. Requires nightly.
@ci: test lint audit
    cargo clippy --all-targets -- -D warnings
    cargo test --doc
    cargo build --manifest-path tests/no_std_build/Cargo.toml

# Install required tools
setup:
    #!/usr/bin/env bash
    if [[ -z $(which cargo) ]]; then
    	printf "Installing 🦀 {{ BOLD }}{{ RED }}Rust{{ RESET }}...\n"
    	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    else
        rustup update
    fi
    brew tap ceejbot/tap
    brew install cargo-nextest tomato semver-bump cargo-audit cargo-llvm-cov
    rustup install nightly

# Tag a new version for release.
version BUMP:
    #!/usr/bin/env bash
    set -e
    current=$(tomato get package.version Cargo.toml)
    version=$(semver-bump {{ BUMP }} "$current")
    tomato set package.version "$version" Cargo.toml &> /dev/null
    cargo generate-lockfile
    git commit Cargo.toml -m "v${version}"
    git tag "v${version}"
    printf "Release tagged for version {{ BOLD_BLUE }}v${version}{{ RESET }}\n"

# publish to crates.io
@release:
    cargo publish

RESET := "\\e[0m"
BOLD := "\\e[1m"
BOLD_YELLOW := "\\e[1;33m"
BOLD_BLUE := "\\e[1;34m"
BLACK := "\\e[30m"
BLACK_BG := "\\e[40m"
RED := "\\e[31m"
RED_BG := "\\e[41m"
GREEN := "\\e[32m"
GREEN_BG := "\\e[42m"
YELLOW := "\\e[33m"
YELLOW_BG := "\\e[43"
BLUE := "\\e[34m"
BLUE_BG := "\\e[44m"
MAGENTA := "\\e[35m"
MAGENTA_BG := "\\e[45m"
CYAN := "\\e[36m"
CYAN_BG := "\\e[46m"
WHITE := "\\e[37m"
WHITE_BG := "\\e[47m"
DEFAULT := "\\e[39m"
DEFAULT_BG := "\\e[49m"
