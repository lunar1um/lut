use std::{fs, collections::BTreeMap, error::Error};

pub fn write_index(files: &BTreeMap<String, String>) -> Result<(), Box<dyn Error>> {
    let mut content = String::new();

    for (path, hash) in files {
        content.push_str(&format!("{} {}\n", hash, path));
    }
    fs::write(".lut/index", content)?;

    Ok(())
}

pub fn read_index() -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let content = fs::read_to_string(".lut/index")?;
    let mut index = BTreeMap::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 {
            index.insert(parts[1].to_string(), parts[0].to_string());
        }
    }

    Ok(index)
}
