pub fn github_api_token() -> String {
    let output = std::process::Command::new("git")
        .arg("config")
        .arg("--get")
        .arg("github.oauth-token")
        .output()
        .unwrap();

    if output.status.success() {
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    } else {
        panic!("No GitHub token configured. To configure, run: git config github.oauth-token <your-token>")
    }
}

pub fn atomic_write(dest_path: &std::path::Path, data: &impl serde::Serialize) -> std::io::Result<()> {
    let mut tmp_path = dest_path.to_owned();
    tmp_path.set_extension("tmp");

    let tmp_file = std::fs::File::create(&tmp_path)?;
    serde_json::to_writer(&tmp_file, &data)?;
    tmp_file.sync_all()?;

    std::fs::rename(tmp_path, dest_path)
}
