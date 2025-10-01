use clap::Parser;
use commands::{Cli, Commands};
use flate2::bufread::ZlibDecoder;
use hex;
use sha2::{Digest, Sha256};
use std::{env, error::Error, fs, io::Read, path::PathBuf};

mod commands;
mod index;
mod objects;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Add { path } => {
            if path == ".".to_string() {
                objects::add_files(&env::current_dir()?)?; // add everything from the top directory
            } else {
                objects::add_files(&PathBuf::from(path))?;
            }

            Ok(())
        }
        Commands::Commit => {
            let index_hash = index::read_index()?;

            if index_hash.is_empty() {
                eprintln!("already up-to-date (nothing new was added)");
            }

            // start creating tree from the root
            let tree_hex = objects::create_tree_recur(&index_hash, "")?;

            let parent = fs::read_to_string(".lut/HEAD")
                .ok()
                .filter(|a| !a.trim().is_empty());
            let mut commit_content = format!("tree {}\n", tree_hex);

            if let Some(parent_hash) = parent {
                commit_content.push_str(&format!("parent {}\n", parent_hash.trim()));
            }

            // add author and committer
            let author = "skibidi <hi@gmail.com>";
            let timestamp = 1234456789;
            commit_content.push_str(&format!("author {} {} +0000\n", author, timestamp));
            commit_content.push_str(&format!("committer {} {} +0000\n", author, timestamp));

            // add blank line and commit message
            commit_content.push_str(&format!("\n{}", "initial"));

            // create commit object
            let header = format!("commit {}\0", commit_content.len());
            let store = [header.as_bytes(), commit_content.as_bytes()].concat();

            let mut hasher = Sha256::new();
            hasher.update(&store);
            let hash_bytes = hasher.finalize();
            let hash_hex = hex::encode(hash_bytes);

            objects::save_object(&hash_hex, &store)?;

            // upd HEAD
            fs::write(".lut/HEAD", &hash_hex)?;

            Ok(())
        }
        Commands::Init => {
            let main_directory = env::current_dir()?.join(".lut");

            if !main_directory.exists() {
                fs::create_dir(&main_directory)?;
            }

            let head = main_directory.join("HEAD");
            let objects_directory = main_directory.join("objects");

            fs::write(&head, "")?;
            fs::create_dir(&objects_directory)?;

            Ok(())
        }
        Commands::Log => {
            let head = env::current_dir()?.join(".lut/HEAD");
            let head_hash = fs::read_to_string(head)?.trim().to_string();

            log_commit(&head_hash)?;
            Ok(())
        }
        Commands::Debug { hash } => {
            let path = format!(".lut/objects/{}/{}", &hash[..2], &hash[2..]);

            let compressed = fs::read(path)?;

            let mut decoder = ZlibDecoder::new(&compressed[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;

            let mut parts = decompressed.splitn(2, |&b| b == 0);
            let header = parts.next().unwrap();
            let body = parts.next().unwrap_or(&[]);

            println!("header: {}", String::from_utf8_lossy(header));
            println!("body (hex): {}", hex::encode(body));
            // printing hex to avoid some weird binary
            Ok(())
        }
    }
}

fn log_commit(hash: &str) -> Result<(), Box<dyn Error>> {
    let obj_path = format!(".lut/objects/{}/{}", &hash[..2], &hash[2..]);
    let compressed = fs::read(obj_path)?;
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    let content = String::from_utf8_lossy(&decompressed);
    // VERY IMPORTNAT: SKIP THE FUCKING HEADER
    let after_header = content.split('\0').nth(1).unwrap_or("");

    let mut parent_hash: Option<&str> = None;

    println!("\ncommit {}", hash);

    for line in after_header.lines() {
        if line.starts_with("tree ") {
            // no need to print tree
        } else if line.starts_with("parent ") {
            parent_hash = Some(&line[7..]);
        } else if line.starts_with("author ") {
            println!("author: {}", &line[7..]);
        } else if line.is_empty() {
            // message is after the empty line
            break;
        }
    }

    // print message (everything after the empty line)
    let parts: Vec<&str> = after_header.split("\n\n").collect();
    if parts.len() > 1 {
        println!("\nmessage: {}", parts[1].trim());
    }

    if let Some(hash) = parent_hash {
        log_commit(&hash)?;
    }

    Ok(())
}
