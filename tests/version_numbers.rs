#[test]
fn changelog() {
    version_sync::assert_contains_substring!("CHANGELOG.md", "## [{version}]");
}

#[test]
fn html_root_url() {
    version_sync::assert_html_root_url_updated!("src/lib.rs");
}
