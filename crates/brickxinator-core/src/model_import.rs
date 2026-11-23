//! Bounded inventory extraction from LDraw text. No geometry or network access.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub reference: String,
    pub color: String,
    pub quantity: u64,
}
pub fn ldraw(source: &str) -> Result<Vec<Part>, String> {
    if source.len() > 4 * 1024 * 1024 {
        return Err("model text exceeds 4 MiB".into());
    }
    let mut files = BTreeMap::<String, Vec<(String, String)>>::new();
    let mut current = "__root__".to_string();
    let mut root = None;
    let mut total = 0;
    files.insert(current.clone(), vec![]);
    for raw in source.lines() {
        let fields: Vec<_> = raw.split_whitespace().collect();
        if fields.is_empty() {
            continue;
        }
        if fields.len() >= 3 && fields[0] == "0" && fields[1].eq_ignore_ascii_case("FILE") {
            let name = fields[2..].join(" ").replace('\\', "/").to_lowercase();
            if files.contains_key(&name) {
                return Err("duplicate embedded model name".into());
            }
            root.get_or_insert_with(|| name.clone());
            current = name.clone();
            files.insert(name, vec![]);
            continue;
        }
        if fields[0] == "0"
            && fields
                .get(1)
                .is_some_and(|v| v.eq_ignore_ascii_case("NOFILE"))
        {
            current = "__root__".into();
            continue;
        }
        if fields[0] != "1" {
            if !matches!(fields[0], "0" | "2" | "3" | "4" | "5") {
                return Err("unsupported LDraw line type".into());
            }
            continue;
        }
        if fields.len() < 15 {
            return Err("incomplete LDraw part reference".into());
        }
        for value in &fields[2..14] {
            let number = value
                .parse::<f64>()
                .map_err(|_| "invalid model transform")?;
            if !number.is_finite() {
                return Err("nonfinite model transform".into());
            }
        }
        total += 1;
        if total > 100_000 {
            return Err("too many model references".into());
        }
        let name = fields[14..].join(" ").replace('\\', "/").to_lowercase();
        files
            .get_mut(&current)
            .ok_or("missing model section")?
            .push((name, fields[1].into()));
    }
    fn visit(
        name: &str,
        color: &str,
        files: &BTreeMap<String, Vec<(String, String)>>,
        path: &mut Vec<String>,
        out: &mut BTreeMap<(String, String), u64>,
        expanded: &mut u64,
    ) -> Result<(), String> {
        if path.len() >= 64 || path.iter().any(|n| n == name) {
            return Err("cyclic or excessively nested model references".into());
        }
        path.push(name.into());
        for (reference, local_color) in files.get(name).ok_or("model section does not exist")? {
            *expanded += 1;
            if *expanded > 1_000_000 {
                return Err("expanded model exceeds one million references".into());
            }
            let color = if local_color == "16" {
                color
            } else {
                local_color
            };
            if reference.ends_with(".dat") {
                let count = out.entry((reference.clone(), color.into())).or_default();
                *count = count.checked_add(1).ok_or("part count overflow")?;
            } else if files.contains_key(reference) {
                visit(reference, color, files, path, out, expanded)?;
            } else {
                return Err(format!("referenced submodel is not embedded: {reference}"));
            }
        }
        path.pop();
        Ok(())
    }
    let root = root.unwrap_or_else(|| "__root__".into());
    let mut out = BTreeMap::new();
    visit(&root, "16", &files, &mut vec![], &mut out, &mut 0)?;
    if out.is_empty() {
        return Err("model contains no inventory part references".into());
    }
    Ok(out
        .into_iter()
        .map(|((reference, color), quantity)| Part {
            reference,
            color,
            quantity,
        })
        .collect())
}
