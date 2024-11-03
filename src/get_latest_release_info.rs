use std::env::consts::{ARCH, OS};

use anyhow::{bail, Result};
use phf::phf_map;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Release {
    pub id: u64,
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: u64,
    pub url: String,
    pub size: u64,
    pub name: String,
    pub browser_download_url: String,
}

impl Asset {
    pub fn get_asset_info(&self) -> AssetInfo {
        let parts = self.name.split("-").collect::<Vec<_>>();
        assert!(parts[0] == "agda");
        let version = parts[1].to_owned();
        let arch = parts[2].to_owned();
        let os = parts
            .iter()
            .skip(3)
            .take_while(|p| !p.starts_with("ghc"))
            .map(|s| *s)
            .collect::<Vec<_>>()
            .join("-");
        let ghc = parts[3 + os.split("-").count()]
            .trim_start_matches("ghc")
            .to_owned();
        AssetInfo {
            arch,
            version,
            os,
            ghc,
        }
    }
}

#[derive(Debug)]
pub struct AssetInfo {
    pub arch: String,
    pub version: String,
    pub os: String,
    pub ghc: String,
}

static ALLOWED_ARCHS: phf::Map<&'static str, &'static [&'static str]> = phf_map! {
    "x86_64" => &["x64"],
    "aarch64" => &["arm64"],
};

static ALLOWED_OSS: phf::Map<&'static str, &'static [&'static str]> = phf_map! {
    "macos" => &["macos"],
    "windows" => &["windows"],
};

impl AssetInfo {
    pub fn applies_to_this_machine(&self) -> bool {
        let allowed_archs = match ALLOWED_ARCHS.get(&ARCH) {
            Some(v) => v,
            None => return false,
        };

        if !allowed_archs.contains(&self.arch.as_str()) {
            return false;
        }

        let parts = self.os.split("-").collect::<Vec<_>>();

        let allowed_oss = match ALLOWED_OSS.get(&OS) {
            Some(v) => v,
            None => return false,
        };

        if !allowed_oss.contains(&parts[0]) {
            return false;
        }

        // Any more?
        return true;
    }
}

pub async fn get_latest_github_release_info(client: &Client) -> Result<Release> {
    let resp = client
        .get("https://api.github.com/repos/wenkokke/setup-agda/releases")
        .send()
        .await?;
    let data: Vec<Release> = resp.json().await?;

    let latest_release = match data.into_iter().find(|r| r.tag_name == "latest") {
        Some(v) => v,
        None => bail!("could not find latest release"),
    };

    Ok(latest_release)
}
