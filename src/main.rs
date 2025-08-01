use argh::FromArgs;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::{collections::HashMap, path::PathBuf};

/// locker - a tool to lint your flake.lock file
#[derive(FromArgs)]
#[argh(help_triggers("-h", "--help"))]
struct Args {
    #[argh(positional, default = "PathBuf::from(\"flake.lock\")")]
    flake_lock: PathBuf,
}

#[derive(Deserialize, Debug)]
struct FlakeLock {
    nodes: HashMap<String, Node>,
    version: usize,

    #[allow(dead_code)]
    root: String,
}

#[derive(Deserialize, Debug)]
struct Node {
    locked: Option<Locked>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Locked {
    // scm
    GitHub { owner: String, repo: String },
    GitLab { owner: String, repo: String },
    SourceHut { owner: String, repo: String },

    // url
    Git { url: String },
    Hg { url: String },
    Tarball { url: String },

    // path
    Path { path: String },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    let flake_lock_content = fs::read_to_string(&args.flake_lock)?;
    let flake_lock: FlakeLock = serde_json::from_str(&flake_lock_content)?;

    if flake_lock.version != 7 {
        eprintln!("Unsupported flake.lock version: {}", flake_lock.version);
        std::process::exit(1);
    }

    let inputs = parse_inputs(flake_lock);
    let duplicates = find_duplicates(inputs);

    if duplicates.is_empty() {
        println!("No duplicate inputs found.");
        std::process::exit(0);
    }

    println!("The following flake uris contained duplicate entries in your flake.lock:");
    for (input, dups) in duplicates {
        eprintln!("  '{}': {}", input, dups.join(", "));
    }

    std::process::exit(1);
}

fn parse_inputs(flake_lock: FlakeLock) -> HashMap<String, String> {
    let mut data = HashMap::new();

    for (k, v) in flake_lock.nodes {
        if v.locked.is_none() {
            continue;
        }

        let val = flake_uri(v.locked.unwrap());
        data.entry(k).insert_entry(val);
    }

    data
}

fn find_duplicates(inputs: HashMap<String, String>) -> HashMap<String, Vec<String>> {
    let mut seen: Vec<String> = Vec::new();
    let mut duplicates: HashMap<String, Vec<String>> = HashMap::new();

    for (input_name, input_uri) in inputs {
        if seen.contains(&input_uri) {
            duplicates.entry(input_uri).or_default().push(input_name);
        } else {
            seen.push(input_uri);
        }
    }

    duplicates
}

fn flake_uri(lock: Locked) -> String {
    match lock {
        Locked::GitHub { owner, repo } => make_scm_uri("github", &owner, &repo),
        Locked::GitLab { owner, repo } => make_scm_uri("gitlab", &owner, &repo),
        Locked::SourceHut { owner, repo } => make_scm_uri("sourcehut", &owner, &repo),
        Locked::Git { url } => make_url_uri("git", &url),
        Locked::Hg { url } => make_url_uri("hg", &url),
        Locked::Tarball { url } => make_url_uri("tarball", &url),
        Locked::Path { path } => format!("path:{path}"),
    }
}

fn make_scm_uri(node_type: &str, owner: &str, repo: &str) -> String {
    format!(
        "{node_type}:{}/{}",
        owner.to_lowercase(),
        repo.to_lowercase()
    )
}

fn make_url_uri(node_type: &str, url: &str) -> String {
    format!("{node_type}:{url}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAKE_LOCK: &str = r#"
    {
        "nodes": {
            "input1": {
                "locked": {
                    "type": "github",
                    "owner": "user1",
                    "repo": "repo1"
                }
            },
            "input2": {
                "locked": {
                    "type": "github",
                    "owner": "user2",
                    "repo": "repo2"
                }
            },
            "input3": {
                "locked": {
                    "type": "github",
                    "owner": "user1",
                    "repo": "repo1"
                }
            },
            "input4": {
                "locked": {
                    "type": "git",
                    "url": "https://example.com/repo.git"
                }
            },
            "input5": {
                "locked": {
                    "type": "git",
                    "url": "https://example.com/repo.git"
                }
            }
        },
        "version": 7,
        "root": "."
    }
    "#;

    #[test]
    fn test_parse_inputs() {
        let flake_lock: FlakeLock = serde_json::from_str(FLAKE_LOCK).unwrap();
        let inputs = parse_inputs(flake_lock);

        assert_eq!(inputs.len(), 5);
        assert!(inputs.contains_key("input1"));
        assert!(inputs.contains_key("input2"));
        assert!(inputs.contains_key("input3"));
        assert!(inputs.contains_key("input4"));
        assert!(inputs.contains_key("input5"));

        assert_eq!(inputs.get("input1").unwrap(), "github:user1/repo1");
        assert_eq!(inputs.get("input2").unwrap(), "github:user2/repo2");
        assert_eq!(inputs.get("input3").unwrap(), "github:user1/repo1");
        assert_eq!(
            inputs.get("input4").unwrap(),
            "git:https://example.com/repo.git"
        );
        assert_eq!(
            inputs.get("input5").unwrap(),
            "git:https://example.com/repo.git"
        );
    }

    #[test]
    fn test_duplicates() {
        let flake_lock: FlakeLock = serde_json::from_str(FLAKE_LOCK).unwrap();

        let inputs = parse_inputs(flake_lock);
        let duplicates = find_duplicates(inputs.clone());

        assert_eq!(duplicates.len(), 2);
    }

    #[test]
    fn test_duplicates_2() -> Result<(), Box<dyn Error>> {
        let flake_lock_contents = fs::read_to_string("test/flake-lock.json")?;
        let flake_lock: FlakeLock = serde_json::from_str(&flake_lock_contents)?;

        let inputs = parse_inputs(flake_lock);
        let duplicates = find_duplicates(inputs);

        assert_eq!(duplicates.len(), 13);
        assert!(duplicates.contains_key("github:nixos/nixpkgs"));
        assert_eq!(duplicates.get("github:nixos/nixpkgs").unwrap().len(), 6);

        assert_eq!(
            duplicates
                .get("tarball:https://api.flakehub.com/f/pinned/edolstra/flake-compat/1.0.1/018afb31-abd1-7bff-a5e4-cff7e18efb7a/source.tar.gz")
                .unwrap()
                .len(),
            1
        );

        Ok(())
    }
}
