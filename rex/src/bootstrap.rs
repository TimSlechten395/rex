// use previous compiler for bootstrapping

use anyhow::anyhow;
use std::cell::LazyCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

use crate::data::expr::Expr;

const PREV_COMPILER: LazyCell<Command> = LazyCell::new(|| {
    let conf = read_kv_file("BOOTSTRAP.txt").unwrap();
    let version = conf
        .get("min_version")
        .ok_or(anyhow!("failed to read file version"))
        .unwrap();
    // TODO: semver check
    let mut comp_path = PathBuf::from("versions");
    comp_path.push(version);

    std::process::Command::new(&comp_path)
});

fn read_kv_file(path: &str) -> anyhow::Result<HashMap<String, String>> {
    let contents = fs::read_to_string(path)?;
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

fn compile_min_version(code: &str) -> anyhow::Result<Expr> {
    PREV_COMPILER.arg("code")
}
