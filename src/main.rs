#[macro_use]
extern crate serde;

mod get_latest_release_info;

use std::{
    fs::{self, File, Permissions},
    io::Write,
    os::unix::fs::PermissionsExt,
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use get_latest_release_info::{Asset, AssetInfo};
use serde_json::Value;

#[derive(Debug, Parser)]
struct Opt {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Show,
    Update,
    Install,
    List {
        /// List available versions
        #[clap(short, long)]
        available: bool,
    },
}

// Rough layout

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::parse();
    println!("Hello, world!");

    let client = {
        use reqwest::{
            header::{HeaderMap, HeaderValue},
            ClientBuilder,
        };
        let mut h = HeaderMap::new();
        h.insert("User-Agent", HeaderValue::from_str("mzhang28-agdaup")?);
        ClientBuilder::new().default_headers(h).build()?
    };

    let bin_dir = dirs::executable_dir();

    let data_dir = match dirs::data_dir() {
        Some(v) => v.join("agdaup"),
        None => bail!("no data dir?"),
    };

    let my_bin_dir = data_dir.join("bin");

    match opt.command {
        Command::Show => todo!(),
        Command::Update => todo!(),
        Command::Install => {
            let release_info =
                get_latest_release_info::get_latest_github_release_info(&client).await?;

            let mut assets = release_info
                .assets
                .iter()
                .filter(|asset| asset.get_asset_info().applies_to_this_machine())
                .collect::<Vec<_>>();

            assets.sort_by_key(|asset| &asset.name);

            let desired_asset = assets[assets.len() - 1];
            let asset_info = desired_asset.get_asset_info();

            let version_dir = data_dir.join("versions").join(asset_info.version);
            fs::remove_dir_all(&version_dir).context("could not remove version dir")?;
            fs::create_dir_all(&version_dir).context("could not create version dir")?;

            println!("Downloading {}", desired_asset.browser_download_url);

            let resp = client
                .get(&desired_asset.browser_download_url)
                .send()
                .await
                .context("could not get url")?;

            let download_path = version_dir.join("download.zip");

            let mut stream = resp.bytes_stream();

            {
                let mut file = File::create(&download_path)?;
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    file.write_all(&chunk)?;
                }
            }

            {
                let file = File::open(&download_path).context("could not open downloaded file")?;
                zip_extract::extract(&file, &version_dir, true).context("could not extract")?;
            }

            #[cfg(unix)]
            {
                let agda_bin = version_dir.join("bin").join("agda");
                let meta = fs::metadata(&agda_bin).context("could not get metadata")?;
                let mut perm = meta.permissions();
                perm.set_mode(perm.mode() | 0o100);
                fs::set_permissions(&agda_bin, perm).context("could not set permission")?;

                fs::create_dir_all(&my_bin_dir).context("could not create bin dir")?;
                fs::remove_file(my_bin_dir.join("agda"))
                    .context("could not remove existing symlink")?;
                std::os::unix::fs::symlink(&agda_bin, my_bin_dir.join("agda"))
                    .context("could not symlink")?;

                if let Some(bin_dir) = bin_dir {
                    fs::remove_file(bin_dir.join("agda"));
                    std::os::unix::fs::symlink(&agda_bin, bin_dir.join("agda"))?;
                }
            }
        }
        Command::List { available } => todo!(),
    }

    Ok(())
}
