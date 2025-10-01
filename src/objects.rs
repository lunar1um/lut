use crate::index;
use flate2::{Compression, write::ZlibEncoder};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    error::Error,
    fs,
    io::Write,
    path::PathBuf,
};

pub fn add_files(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let mut index_hash: BTreeMap<String, String> = BTreeMap::new();
    // path: hash

    recur(&path, &mut index_hash)?;
    index::write_index(&index_hash)?;

    fn recur(
        path: &PathBuf,
        index_hash: &mut BTreeMap<String, String>,
    ) -> Result<(), Box<dyn Error>> {
        if path.is_dir() {
            for entry in path.read_dir()? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() && path.file_name().unwrap() != ".lut" {
                    recur(&path, index_hash)?;
                } else {
                    let content = fs::read(path)?;
                    let header = format!("blob {}\0", content.len());
                    let store = [header.as_bytes(), &content].concat();
                    // concatenate the header with the content of the file

                    let mut hasher = Sha256::new();
                    hasher.update(&store);
                    let hash_bytes = hasher.finalize();
                    // hash the combined store with sha-256

                    let hash_hex = hex::encode(hash_bytes);
                    // convert it into hex

                    if save_object(&hash_hex, &store)? {
                        index_hash.insert(entry.path().to_str().unwrap().to_string(), hash_hex);
                    }
                }
            }
        }

        Ok(())
    }

    Ok(())
}

pub fn save_object(hash_hex: &str, store: &Vec<u8>) -> Result<bool, Box<dyn Error>> {
    let blob_path = PathBuf::from(format!(
        ".lut/objects/{}/{}",
        &hash_hex[..2],
        &hash_hex[2..]
    ));

    // if the path with the same hash doesnt exist (or the file was changed)
    if !blob_path.exists() {
        let compressed = zlib_compress(&store)?;
        // compress with zlib like git

        fs::create_dir_all(format!(".lut/objects/{}", &hash_hex[..2]))?;
        // create folder for the blob with the first two characters
        fs::write(blob_path, compressed)?;
        // write the blob file with its content

        Ok(true)
    } else {
        Ok(false)
    }
}

fn zlib_compress(buffer: &Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&buffer)?;
    Ok(encoder.finish()?)
    // no idea whats going on here, just followed the docs
}

pub fn create_tree_recur(
    files: &BTreeMap<String, String>,
    prefix: &str,
) -> Result<String, Box<dyn Error>> {
    let mut tree_content: Vec<u8> = Vec::new();
    let mut sub_directories = HashSet::new();

    for (path, hash) in files {
        if !path.starts_with(prefix) {
            continue;
        } // avoid the same path

        let relative = &path[prefix.len()..];
        // relative current dir, a/b/c/d -> d for example

        if let Some(slash_position) = relative.find("/") {
            // is a sub directory
            let subdir = &relative[..slash_position];
            sub_directories.insert(subdir);
        } else {
            // in this directory
            tree_content.extend(format!("100644 {}\0", relative).as_bytes());
            // 100644 {name}\0{hash_hex} for files
            tree_content.extend(&hex::decode(hash)?);
        }
    }

    // create trees for subdirs
    for subdir in sub_directories {
        let sub_prefix = format!("{}{}/", prefix, subdir);
        let sub_tree_hash = create_tree_recur(files, &sub_prefix)?;

        tree_content.extend(format!("040000 {}\0", subdir).as_bytes());
        // 040000 {name}\0{hash_hex} for folders | directories
        tree_content.extend(&hex::decode(&sub_tree_hash)?);
    }

    let header = format!("tree {}\0", tree_content.len());
    let store = [header.as_bytes(), &tree_content].concat();

    let mut hasher = Sha256::new();
    hasher.update(&store);
    let hash_bytes = hasher.finalize();
    let hash_hex = hex::encode(hash_bytes);

    save_object(&hash_hex, &store)?;

    Ok(hash_hex)
}
