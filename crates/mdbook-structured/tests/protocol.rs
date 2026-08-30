use assert_cmd::Command;

fn command(args: &[&str]) -> Command {
    let mut command = Command::cargo_bin("mdbook-structured").unwrap();
    command.args(args);
    command
}

#[test]
fn capability_render_supports_html() {
    command(&["render", "supports", "html"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn capability_rewrite_links_supports_html() {
    command(&["rewrite-links", "supports", "html"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn capability_rejects_non_html_renderers_without_reading_stdin() {
    command(&["render", "supports", "pdf"])
        .write_stdin("{")
        .assert()
        .failure()
        .stdout("");
}

#[test]
fn capability_rejects_malformed_command_shapes_without_reading_stdin() {
    for arguments in [
        &[][..],
        &["render"][..],
        &["render", "supports"][..],
        &["render", "supports", "html", "extra"][..],
        &["unknown", "supports", "html"][..],
        &["install"][..],
    ] {
        command(arguments)
            .write_stdin("{")
            .assert()
            .failure()
            .stdout("");
    }
}
