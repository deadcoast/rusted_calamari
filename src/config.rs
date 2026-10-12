//! Category table. A category is a directory name with a colour id (cid),
//! a hier tier and an optional hotkey. Loaded from `<root>/.calamari`:
//!
//! ```text
//! # key   name     cid      tier
//! spc     _float_  #E6D463  a
//! -       _fav_    #F0A35E  b
//! z       1        #e56399  c
//! ```

use std::fs;
use std::path::Path;

pub type Rgb = (u8, u8, u8);

#[derive(Clone, Debug)]
pub struct Cat {
    pub key: Option<String>,
    pub name: String,
    pub cid: Rgb,
    pub tier: char,
}

pub fn hex(s: &str) -> Option<Rgb> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn cat(key: &str, name: &str, cid: &str, tier: char) -> Cat {
    Cat {
        key: (key != "-").then(|| key.to_string()),
        name: name.to_string(),
        cid: hex(cid).expect("default cid"),
        tier,
    }
}

/// Defaults from the request. `_fav_` has no key or cid in the request;
/// its cid here is an invention.
pub fn defaults() -> Vec<Cat> {
    vec![
        cat("spc", "_float_", "#E6D463", 'a'),
        cat("-", "_fav_", "#F0A35E", 'b'),
        cat("z", "1", "#e56399", 'c'),
        cat("x", "2", "#92CC7C", 'c'),
        cat("c", "3", "#7f96ff", 'c'),
        cat("v", "4", "#a6cfd5", 'c'),
    ]
}

pub fn load(root: &Path) -> Vec<Cat> {
    let Ok(text) = fs::read_to_string(root.join(".calamari")) else {
        return defaults();
    };
    let cats = parse(&text);
    if cats.is_empty() { defaults() } else { cats }
}

pub fn to_text(cats: &[Cat]) -> String {
    let mut s = String::from("# key   name     cid      tier\n");
    for c in cats {
        s.push_str(&format!(
            "{:<7} {:<8} #{:02X}{:02X}{:02X}  {}\n",
            c.key.as_deref().unwrap_or("-"),
            c.name,
            c.cid.0,
            c.cid.1,
            c.cid.2,
            c.tier
        ));
    }
    s
}

pub fn parse(text: &str) -> Vec<Cat> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            let [key, name, cid, tier] = f[..] else { return None };
            Some(Cat {
                key: (key != "-").then(|| key.to_string()),
                name: name.to_string(),
                cid: hex(cid)?,
                tier: tier.chars().next()?,
            })
        })
        .collect()
}

pub fn save(root: &Path, cats: &[Cat]) -> std::io::Result<()> {
    fs::write(root.join(".calamari"), to_text(cats))
}
