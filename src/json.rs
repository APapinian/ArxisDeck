// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! Small helpers over serde_json for the flat string objects sent to QML.

use serde_json::{Map, Value};

/// `{"k": "v", ...}` with string values.
pub fn object(pairs: &[(&str, &str)]) -> String {
    let map: Map<String, Value> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
        .collect();
    Value::Object(map).to_string()
}

/// `[a, b, ...]` from already-serialised JSON values.
pub fn array(items: &[String]) -> String {
    format!("[{}]", items.join(", "))
}

/// A string as a JSON string literal, without the surrounding quotes.
pub fn escape(s: &str) -> String {
    let quoted = Value::String(s.to_string()).to_string();
    quoted[1..quoted.len() - 1].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_and_builds() {
        assert_eq!(object(&[("a", "x\"y"), ("b", "1\n2")]), r#"{"a":"x\"y","b":"1\n2"}"#);
        assert_eq!(array(&["{}".into(), "{}".into()]), "[{}, {}]");
        assert_eq!(escape("a\"b\\c"), r#"a\"b\\c"#);
    }
}
