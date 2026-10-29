//! Machine-readable output: JSON and SARIF 2.1.0.

use std::path::Path;

use condor_core::paths::format_path;
use condor_core::{Item, ItemKind};
use serde_json::{Value, json};

fn path(item: &Item, cwd: &Path) -> String {
    format_path(&item.file, cwd).to_string_lossy().into_owned()
}

/// One object per finding.
pub fn json(items: &[&Item], cwd: &Path) -> String {
    let findings: Vec<Value> = items
        .iter()
        .map(|i| {
            json!({
                "path": path(i, cwd),
                "line": i.first_line,
                "end_line": i.last_line,
                "kind": i.kind.as_str(),
                "code": i.kind.noqa_code(),
                "name": i.name,
                "message": i.message(),
                "confidence": i.confidence,
                "size": i.size(),
            })
        })
        .collect();
    serde_json::to_string_pretty(&findings).expect("serializable")
}

/// A SARIF 2.1.0 log with one rule per kind.
pub fn sarif(items: &[&Item], cwd: &Path) -> String {
    let rules: Vec<Value> = ItemKind::ALL
        .iter()
        .map(|k| {
            json!({
                "id": k.noqa_code(),
                "name": k.as_str(),
                "shortDescription": {"text": format!("unused {}", k.as_str().replace('_', " "))},
            })
        })
        .collect();
    let results: Vec<Value> = items
        .iter()
        .map(|i| {
            json!({
                "ruleId": i.kind.noqa_code(),
                "level": "warning",
                "message": {"text": format!("{} ({}% confidence)", i.message(), i.confidence)},
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {"uri": path(i, cwd).replace('\\', "/")},
                        "region": {"startLine": i.first_line, "endLine": i.last_line},
                    }
                }],
                "properties": {"confidence": i.confidence},
            })
        })
        .collect();
    let log = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {"driver": {
                "name": "condor",
                "version": env!("CARGO_PKG_VERSION"),
                "informationUri": "https://github.com/mde-pach/condor",
                "rules": rules,
            }},
            "results": results,
        }],
    });
    serde_json::to_string_pretty(&log).expect("serializable")
}
