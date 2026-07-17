use anyhow::{bail, Context};
use serde::Deserialize;
use std::io::Read;
use std::{env, fs};

const REPO: &str = "tjklint/todork";
const CURRENT_VERSION: &str = env!("TODORK_VERSION");

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub fn run() -> anyhow::Result<()> {
    eprintln!("Checking for updates...");

    let release = fetch_latest_release()?;
    let latest = release.tag_name.trim_start_matches('v').to_owned();

    if latest == CURRENT_VERSION {
        println!("todork {CURRENT_VERSION} is already up to date.");
        return Ok(());
    }

    println!("Update available: {CURRENT_VERSION} → {latest}");

    let asset_name = asset_name_for_platform(&latest)?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .with_context(|| {
            format!("no release asset found for this platform ({asset_name}); download manually from https://github.com/{REPO}/releases")
        })?;

    println!("Downloading {asset_name}...");
    let data = download(&asset.browser_download_url)?;

    println!("Extracting...");
    let binary = extract_binary(&data, &asset_name)?;

    let exe = env::current_exe().context("could not determine current executable path")?;
    install_binary(&binary, &exe)?;

    println!("Upgraded to todork {latest}");
    Ok(())
}

fn fetch_latest_release() -> anyhow::Result<Release> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    ureq::get(&url)
        .set("User-Agent", &format!("todork/{CURRENT_VERSION}"))
        .call()
        .context("failed to reach GitHub API")?
        .into_json::<Release>()
        .context("unexpected response from GitHub API")
}

fn download(url: &str) -> anyhow::Result<Vec<u8>> {
    let resp = ureq::get(url)
        .set("User-Agent", &format!("todork/{CURRENT_VERSION}"))
        .call()
        .context("failed to download release asset")?;
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .context("download interrupted")?;
    Ok(buf)
}

fn asset_name_for_platform(version: &str) -> anyhow::Result<String> {
    let target = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        (os, arch) => bail!("unsupported platform: {os}/{arch}"),
    };
    let ext = if env::consts::OS == "windows" {
        "zip"
    } else {
        "tar.gz"
    };
    Ok(format!("todork-{version}-{target}.{ext}"))
}

fn extract_binary(data: &[u8], asset_name: &str) -> anyhow::Result<Vec<u8>> {
    if asset_name.ends_with(".tar.gz") {
        extract_from_tarball(data)
    } else if asset_name.ends_with(".zip") {
        extract_from_zip(data)
    } else {
        bail!("unknown archive format: {asset_name}")
    }
}

fn extract_from_tarball(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let gz = GzDecoder::new(data);
    let mut archive = Archive::new(gz);

    for entry in archive.entries().context("failed to read tar archive")? {
        let mut entry = entry.context("corrupt tar entry")?;
        let path = entry.path().context("invalid path in archive")?;
        let is_todork = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "todork")
            .unwrap_or(false);
        if is_todork {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }

    bail!("todork binary not found in tarball")
}

fn extract_from_zip(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    use std::io::Cursor;
    use zip::ZipArchive;

    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor).context("failed to open zip archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("corrupt zip entry")?;
        let is_todork = file.name() == "todork.exe" || file.name() == "todork";
        if is_todork {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }

    bail!("todork binary not found in zip")
}

fn install_binary(binary: &[u8], target: &std::path::Path) -> anyhow::Result<()> {
    let temp = target.with_extension("update");
    fs::write(&temp, binary).context("failed to write new binary")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp, fs::Permissions::from_mode(0o755))?;
    }

    match fs::rename(&temp, target) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Windows locks the running exe; leave the new binary in place and
            // print instructions instead of erroring out.
            eprintln!("Could not replace binary automatically: {e}");
            eprintln!("New binary saved to: {}", temp.display());
            eprintln!(
                "Replace manually:  move \"{}\" \"{}\"",
                temp.display(),
                target.display()
            );
            Ok(())
        }
    }
}
