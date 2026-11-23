//! Offline interchange adapters. External identifiers are never local UUIDs.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}
pub fn delimited(source: &str, delimiter: char) -> Result<Table, String> {
    if source.len() > 4 * 1024 * 1024 || !matches!(delimiter, ',' | '\t') {
        return Err("file exceeds 4 MiB or delimiter is unsupported".into());
    }
    let mut records = vec![];
    let mut row = vec![];
    let mut field = String::new();
    let mut quoted = false;
    let mut ended = false;
    let mut chars = source.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                    ended = true;
                }
            } else {
                field.push(ch);
            }
        } else if ch == '"' {
            if !field.is_empty() || ended {
                return Err("unexpected quote in delimited field".into());
            }
            quoted = true;
        } else if ch == delimiter {
            row.push(std::mem::take(&mut field));
            ended = false;
        } else if ch == '\n' || ch == '\r' {
            if ch == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            row.push(std::mem::take(&mut field));
            if row.iter().any(|v| !v.is_empty()) {
                records.push(std::mem::take(&mut row));
            } else {
                row.clear();
            }
            ended = false;
        } else {
            if ended {
                return Err("unexpected data after quoted field".into());
            }
            field.push(ch);
        }
        if field.len() > 32768 || row.len() > 128 || records.len() > 100000 {
            return Err("file row, field or column limit exceeded".into());
        }
    }
    if quoted {
        return Err("unterminated quoted field".into());
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        records.push(row);
    }
    if records.is_empty() {
        return Err("file has no header row".into());
    }
    let headers = records.remove(0);
    let mut seen = BTreeSet::new();
    if headers
        .iter()
        .any(|h| h.trim().is_empty() || !seen.insert(h.to_lowercase()))
    {
        return Err("blank or duplicate column headings".into());
    }
    if records.iter().any(|row| row.len() != headers.len()) {
        return Err("delimited rows have inconsistent column counts".into());
    }
    Ok(Table {
        headers,
        rows: records,
    })
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Columns {
    pub part: String,
    pub color: String,
    pub quantity: String,
    pub inventory: Option<String>,
    pub inventory_id: Option<String>,
    pub spare: Option<String>,
    pub include_spares: bool,
}
pub fn bom(table: &Table, columns: &Columns) -> Result<Vec<crate::model_import::Part>, String> {
    let index = |name: &str| {
        table
            .headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| format!("missing column {name}"))
    };
    let part = index(&columns.part)?;
    let color = index(&columns.color)?;
    let quantity = index(&columns.quantity)?;
    let inventory = columns.inventory.as_deref().map(index).transpose()?;
    let spare = columns.spare.as_deref().map(index).transpose()?;
    if inventory.is_some()
        && columns
            .inventory_id
            .as_ref()
            .is_none_or(|id| id.trim().is_empty())
    {
        return Err(
            "select an explicit Rebrickable inventory ID before importing bulk inventory parts"
                .into(),
        );
    }
    let mut parts = BTreeMap::<(String, String), u64>::new();
    for row in &table.rows {
        if inventory.is_some_and(|i| columns.inventory_id.as_deref() != Some(&row[i])) {
            continue;
        }
        if let Some(i) = spare {
            match row[i].to_lowercase().as_str() {
                "t" | "true" | "1" if !columns.include_spares => continue,
                "t" | "true" | "1" | "f" | "false" | "0" => {}
                _ => return Err("invalid spare flag".into()),
            }
        }
        let n = row[quantity]
            .parse::<u64>()
            .map_err(|_| "quantity must be a nonnegative whole count")?;
        if row[part].trim().is_empty() || row[color].trim().is_empty() {
            return Err("part and color identifiers are required".into());
        }
        if n == 0 {
            continue;
        }
        let sum = parts
            .entry((row[part].clone(), row[color].clone()))
            .or_default();
        *sum = sum.checked_add(n).ok_or("BOM quantity overflow")?;
    }
    if parts.is_empty() {
        return Err("selected inventory has no positive part quantities".into());
    }
    Ok(parts
        .into_iter()
        .map(|((reference, color), quantity)| crate::model_import::Part {
            reference,
            color,
            quantity,
        })
        .collect())
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wanted {
    pub part_id: String,
    pub color_id: u32,
    pub quantity: u64,
}
pub fn wanted_xml(items: &[Wanted]) -> Result<String, String> {
    if items.is_empty() || items.len() > 100000 {
        return Err("choose a bounded nonempty wanted list".into());
    }
    let mut rows = BTreeMap::<(String, u32), u64>::new();
    for item in items {
        if item.part_id.is_empty()
            || item.part_id.len() > 128
            || item.part_id.chars().any(|c| c.is_control())
            || item.quantity == 0
        {
            return Err(
                "explicit BrickLink part/color mapping and positive quantity are required".into(),
            );
        }
        let n = rows
            .entry((item.part_id.clone(), item.color_id))
            .or_default();
        *n = n
            .checked_add(item.quantity)
            .ok_or("wanted quantity overflow")?;
    }
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<INVENTORY>\n");
    for ((id, color), qty) in rows {
        let id = id
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;");
        xml.push_str(&format!("  <ITEM><ITEMTYPE>P</ITEMTYPE><ITEMID>{id}</ITEMID><COLOR>{color}</COLOR><MINQTY>{qty}</MINQTY></ITEM>\n"));
    }
    xml.push_str("</INVENTORY>\n");
    Ok(xml)
}
