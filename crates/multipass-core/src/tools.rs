use crate::ncfile::NcFile;
use crate::pycompat::is_space;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap, HashSet};

pub type ToolMap = BTreeMap<u64, u64>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryEntry {
    pub description: String,
    pub number: u64,
}

pub fn normalize_description(d: &str) -> String {
    d.split(is_space).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ").to_lowercase()
}

pub fn default_tool_map<F: Borrow<NcFile>>(files: &[F]) -> Vec<ToolMap> {
    assign_tools(files, &[])
}

pub fn assign_tools<F: Borrow<NcFile>>(files: &[F], inventory: &[InventoryEntry]) -> Vec<ToolMap> {
    let inv: HashMap<String, u64> =
        inventory.iter().map(|e| (normalize_description(&e.description), e.number)).collect();
    let mut taken: HashSet<u64> = inventory.iter().map(|e| e.number).collect();
    let mut by_desc: HashMap<String, u64> = HashMap::new();
    let mut maps = Vec::with_capacity(files.len());
    for f in files.iter().map(Borrow::borrow) {
        let mut m = ToolMap::new();
        for &n in &f.tool_numbers {
            let desc = f.tool(n).map(str::to_owned).unwrap_or_else(|| format!("#{n}"));
            let t = match by_desc.get(&desc) {
                Some(&t) => t,
                None => {
                    let t = match inv.get(&normalize_description(&desc)) {
                        Some(&t) => t,
                        None => {
                            let mut t = 1;
                            while taken.contains(&t) {
                                t += 1;
                            }
                            t
                        }
                    };
                    taken.insert(t);
                    by_desc.insert(desc, t);
                    t
                }
            };
            m.insert(n, t);
        }
        maps.push(m);
    }
    maps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nc(desc: &str) -> NcFile {
        let src = format!("(T1  {desc}  Makera    D=3.)\nG90\nT1M6\nM30\n");
        NcFile::parse("x.nc", src.as_bytes()).unwrap()
    }

    #[test]
    fn gleiches_werkzeug_gleiche_nummer() {
        let files = [nc("A"), nc("B"), nc("A")];
        let maps = default_tool_map(&files);
        assert_eq!(maps[0][&1], 1);
        assert_eq!(maps[1][&1], 2);
        assert_eq!(maps[2][&1], 1);
    }

    #[test]
    fn inventar_hat_vorrang_und_wird_freigehalten() {
        let inv = [
            InventoryEntry { description: "spiral  o metal MAKERA".into(), number: 13 },
            InventoryEntry { description: "Platzhalter".into(), number: 1 },
        ];
        let files = [nc("Neu"), nc("Spiral O Metal"), nc("Noch neuer")];
        let maps = assign_tools(&files, &inv);
        assert_eq!(maps[0][&1], 2);
        assert_eq!(maps[1][&1], 13);
        assert_eq!(maps[2][&1], 3);
    }
}
