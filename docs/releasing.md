# Releasing

In order to release a new version, follow these steps.

1. Ensure the [CHANGELOG.md](../CHANGELOG.md) is up to date.

1. Update the version number in [Cargo.toml](../Cargo.toml).

1. Update the version number in the `html_root_url` tag in [lib.rs](../src/lib.rs).

1. Update the [README.md](../README.md) by running `cargo readme -o README.md` in the root of the repository.

TODO: I think this has to change. cargo-readme doesn't work with feature-gated docs, which is nice to have for the metrics feature. I'd like to have a code example for it, which will only compile with the metrics feature. But I also want cargo test to work with just the default features.

Parsing cfg_attr in cargo-readme seems quite hard, though. So maybe the solution is to create a shorter, separately updated readme. Then have an integration test that extracts code samples from the readme and compiles them to ensure they keep working.


1. Commit the changes.

1. Tag the commit.

1. Run `cargo publish`.

1. Push the commit and the tag.
