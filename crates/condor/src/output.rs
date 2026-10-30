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

/// Percent-encode a path for a URI (RFC 3986): everything but unreserved
/// characters and `/`.
fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The path as a URI path: `/` separators (on Windows), a leading `/` for
/// drive letters.
fn uri_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    let text = if cfg!(windows) { text.replace('\\', "/") } else { text.into_owned() };
    let text = if text.starts_with('/') { text } else { format!("/{text}") };
    encode_path(&text)
}

/// An item's location: relative to `%SRCROOT%` (the working directory) when
/// below it, otherwise an absolute `file:` URI.
fn artifact_location(item: &Item, cwd: &Path) -> Value {
    match item.file.strip_prefix(cwd) {
        Ok(rel) if item.file.is_absolute() => {
            let rel = rel.to_string_lossy();
            let rel = if cfg!(windows) { rel.replace('\\', "/") } else { rel.into_owned() };
            json!({"uri": encode_path(&rel), "uriBaseId": "%SRCROOT%"})
        }
        _ => json!({"uri": format!("file://{}", uri_path(&item.file))}),
    }
}

/// A SARIF 2.1.0 log with one rule per kind. `errors` are the files that
/// could not be analysed.
pub fn sarif(items: &[&Item], cwd: &Path, errors: &[String]) -> String {
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
                        "artifactLocation": artifact_location(i, cwd),
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
            "originalUriBaseIds": {"%SRCROOT%": {"uri": format!("file://{}/", uri_path(cwd).trim_end_matches('/'))}},
            "invocations": [{
                "executionSuccessful": errors.is_empty(),
                "toolExecutionNotifications": errors.iter().map(|e| json!({"level": "error", "message": {"text": e}})).collect::<Vec<_>>(),
            }],
            "results": results,
        }],
    });
    serde_json::to_string_pretty(&log).expect("serializable")
}
