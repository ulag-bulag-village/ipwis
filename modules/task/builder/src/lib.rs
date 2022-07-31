#![feature(exit_status_error)]

use std::{env, fs, path::PathBuf, process::Command};

use ipis::core::anyhow::{bail, Result};

pub type BuildResult = Result<()>;

fn build_wasi(src: &str, dst: &'static str) -> Result<PathBuf> {
    let mut cargo = Command::new("cargo");

    // build
    cargo
        .arg("+nightly")
        .arg("build")
        .arg("--color=always")
        .arg("--target=wasm32-wasi")
        .arg("--target-dir=./target") // allow workspace-level parallel building (avoid from begin locked)
        .arg("--lib")
        .arg("--release")
        .status()?
        .exit_ok()?;

    // resolve the output file
    let src: PathBuf = src.parse()?;

    // move to the out_dir
    let dst: PathBuf = {
        let mut buf: PathBuf = env::var("OUT_DIR")?.parse()?;
        buf.push(dst);
        buf
    };
    fs::copy(&src, &dst)?;
    Ok(dst)
}

pub fn try_build_wasi(src: &str, dst: &'static str) -> BuildResult {
    match ::build_target::target_arch()? {
        ::build_target::Arch::WASM32 => {
            // skipping building itself
            Ok(())
        }
        ::build_target::Arch::Other(arch) => {
            // unsupported architechures
            // NOTE: wasm64 is not supported yet!
            bail!("unsupported architecture: {arch}")
        }
        _ => build_wasi(src, dst).map(|_| ()),
    }
}
