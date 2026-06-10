// use previous compiler for bootstrapping

use anyhow::{Context, anyhow, bail};
use std::cell::LazyCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

use crate::data::expr::Expr;

static PREV_COMPILER: LazyLock<PathBuf> = LazyLock::new(|| {
    let conf = read_kv_file("BOOTSTRAP.txt").unwrap();
    let version = conf
        .get("min_version")
        .ok_or(anyhow!("failed to read file version"))
        .unwrap();
    // TODO: semver check
    let mut comp_path = PathBuf::from("versions");
    comp_path.push(version);
    comp_path
});

fn read_kv_file(path: &str) -> anyhow::Result<HashMap<String, String>> {
    let contents = fs::read_to_string(path).context(format!("failed to read {}", path))?;
    let mut map = HashMap::new();

    for line in contents.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(map)
}

pub fn compile_min_version(code: &str) -> anyhow::Result<Vec<(String, Expr)>> {
    // dbg!(code);
    let output = std::process::Command::new(&LazyLock::force(&PREV_COMPILER))
        .arg("run")
        .arg("-b")
        .arg(code)
        .output()
        .expect("Failed to exec command");

    if output.status.success() {
        let expr = serde_json::from_slice::<Vec<(String, Expr)>>(&output.stdout)
            .context("could not parse json")?;
        Ok(expr)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        bail!("bootstrap compiler crashed \nB: {}", err)
    }
}
