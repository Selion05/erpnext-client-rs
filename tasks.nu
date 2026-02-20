#!/usr/bin/env nu

# Task runner for erpnext-client



# Show all available tasks
def main [] {
    print "Available tasks:"
    print "  test               - Run all tests"
    print "  fmt                - Format code with rustfmt"
    print "  lint               - Run clippy linter"
    print "  check              - Check code without building"
    print "  clean              - Clean build artifacts"
    print "  publish:macro      - Publish erpnext_client_macro to kellnr (optional: --patch, --minor, --major)"
    print "  publish:client     - Publish erpnext_client to kellnr (optional: --patch, --minor, --major)"
}


# Run all tests
def "main test" [...args: string] {
    cargo test ...$args
}

# Format code with rustfmt
def "main fmt" [] {
    cargo fmt --all
}

# Run clippy linter
def "main lint" [] {
    cargo clippy --all-targets --all-features -- -D warnings
}

# Check code without building
def "main check" [] {
    cargo check --all-targets --all-features
}

# Clean build artifacts
def "main clean" [] {
    cargo clean
}

# Publish erpnext_client_macro to kellnr registry
def "main publish:macro" [
    --patch    # Bump patch version (0.1.0 -> 0.1.1)
    --minor    # Bump minor version (0.1.0 -> 0.2.0)
    --major    # Bump major version (0.1.0 -> 1.0.0)
] {
    if $patch or $minor or $major {
        let bump = if $patch { "patch" } else if $major { "major" } else { "minor" }
        cargo set-version -p erpnext_client_macro --bump $bump

        let version = (open macro/Cargo.toml | get package.version)

        git add macro/Cargo.toml Cargo.lock
        git commit -m $"chore: Bump erpnext_client_macro to ($version)"
    }

    cargo publish -p erpnext_client_macro --registry kellnr
}

# Publish erpnext_client to kellnr registry
def "main publish:client" [
    --patch    # Bump patch version (0.1.0 -> 0.1.1)
    --minor    # Bump minor version (0.1.0 -> 0.2.0)
    --major    # Bump major version (0.1.0 -> 1.0.0)
] {
    if $patch or $minor or $major {
        let bump = if $patch { "patch" } else if $major { "major" } else { "minor" }
        cargo set-version -p erpnext_client --bump $bump

        let version = (open client/Cargo.toml | get package.version)

        git add client/Cargo.toml Cargo.lock
        git commit -m $"chore: Bump erpnext_client to ($version)"
    }

    cargo publish -p erpnext_client --registry kellnr
}
