# Releasing

In order to release a new version, follow these steps.

1. Ensure the [CHANGELOG.md](../CHANGELOG.md) is up to date.

1. Update the version number in [Cargo.toml](../Cargo.toml).

1. Update the version number in the `html_root_url` tag in [lib.rs](../src/lib.rs).

1. Commit the changes.

1. Tag the commit.

1. Run `cargo publish`.

1. Push the commit and the tag.
