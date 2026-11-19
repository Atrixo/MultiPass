mod error;
mod merge;
mod ncfile;
mod pycompat;
mod rapid;
mod tools;

pub use error::NcError;
pub use merge::{ascii_safe, merge, MergeOptions, MergeResult, Warning, APP_NAME, APP_VERSION};
pub use ncfile::{NcFile, Units};
pub use pycompat::splitlines;
pub use rapid::{restore_rapids, NonAsciiCode, RapidStats};
pub use tools::{assign_tools, default_inventory, default_tool_map, normalize_description, InventoryEntry, ToolMap};

pub fn natural_key(s: &str) -> Vec<NaturalPart> {
    let mut out = Vec::new();
    let mut digits = String::new();
    let mut text = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            if !text.is_empty() {
                out.push(NaturalPart::Text(std::mem::take(&mut text)));
            }
            digits.push(c);
        } else {
            if !digits.is_empty() {
                out.push(NaturalPart::Number(digits.parse().unwrap_or(u128::MAX)));
                digits.clear();
            }
            text.extend(c.to_lowercase());
        }
    }
    if !digits.is_empty() {
        out.push(NaturalPart::Number(digits.parse().unwrap_or(u128::MAX)));
    }
    if !text.is_empty() {
        out.push(NaturalPart::Text(text));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NaturalPart {
    Number(u128),
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natuerliche_sortierung() {
        let mut v = vec!["10_x.cnc", "2_b.cnc", "1_a.cnc", "B.cnc", "a.cnc"];
        v.sort_by_key(|s| natural_key(s));
        assert_eq!(v, vec!["1_a.cnc", "2_b.cnc", "10_x.cnc", "a.cnc", "B.cnc"]);
    }
}
