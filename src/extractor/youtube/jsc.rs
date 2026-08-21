use crate::core::error::Result;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigOp {
    Reverse,
    Slice(usize),
    Splice(usize),
    Swap(usize),
}

#[derive(Debug, Clone, Default)]
pub struct JsChallengeSolver {
    pub player_url: Option<String>,
    pub sig_ops: Vec<SigOp>,
}

static SIG_FUNC_NAME_RE_1: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"\b[cs]\s*&&\s*[adf]\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*([a-zA-Z0-9$]+)\("#,
    )
    .unwrap()
});

static SIG_FUNC_NAME_RE_2: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b([a-zA-Z0-9$]+)\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#)
        .unwrap()
});

static SIG_FUNC_NAME_RE_3: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"([a-zA-Z0-9$]+)\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\([^)]*\);"#)
        .unwrap()
});

static N_FUNC_NAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?x)
        (?:
            \.get\("n"\)\)&&\(b=([a-zA-Z0-9$]+)(?:\[(\d+)\])?\([a-zA-Z0-9$]+\)|
            (?:b=([a-zA-Z0-9$]+)(?:\[(\d+)\])?\([a-zA-Z0-9$]+\),\s*!b\s*&&)|
            (?:b\s*=\s*String\.fromCharCode\(110\),\s*c\s*=\s*a\.get\(b\)\)\s*&&\s*\(c\s*=\s*([a-zA-Z0-9$]+)(?:\[(\d+)\])?\(c\))
        )
    "#).unwrap()
});

impl JsChallengeSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse_player_js(&mut self, player_js: &str) -> Result<()> {
        self.sig_ops = self.extract_sig_ops(player_js)?;
        Ok(())
    }

    pub fn extract_sig_ops(&self, js: &str) -> Result<Vec<SigOp>> {
        let func_name = SIG_FUNC_NAME_RE_1
            .captures(js)
            .or_else(|| SIG_FUNC_NAME_RE_2.captures(js))
            .or_else(|| SIG_FUNC_NAME_RE_3.captures(js))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str());

        let func_name = match func_name {
            Some(n) => n,
            None => return Ok(Vec::new()),
        };

        let func_decl_pattern = format!(
            r#"(?:var\s+)?{func_name}\s*=\s*function\("#,
            func_name = regex::escape(func_name)
        );
        let decl_re = match Regex::new(&func_decl_pattern) {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let start_pos = match decl_re.find(js) {
            Some(m) => m.start(),
            None => return Ok(Vec::new()),
        };

        let body = match extract_balanced_braces(&js[start_pos..]) {
            Some(b) => b,
            None => return Ok(Vec::new()),
        };

        // Match statements like `ab.rq(a, 12);` or `ab["rq"](a, 12);` or `ab.wx(a);`
        let statement_re = Regex::new(
            r#"([a-zA-Z0-9$]+)(?:\.([a-zA-Z0-9$]+)|\["([^"]+)"\])\s*\([^,)]*(?:,\s*(\d+))?\)"#,
        )
        .unwrap();
        let mut ops = Vec::new();
        let mut object_cache: HashMap<String, HashMap<String, String>> = HashMap::new();

        for cap in statement_re.captures_iter(&body) {
            let obj_name = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            let method_name = cap
                .get(2)
                .or_else(|| cap.get(3))
                .map(|m| m.as_str())
                .unwrap_or_default();
            let arg_val = cap
                .get(4)
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .unwrap_or(0);

            if obj_name == "a" || obj_name == "b" || obj_name == "c" {
                continue;
            }

            if !object_cache.contains_key(obj_name) {
                let methods = parse_helper_object(js, obj_name);
                object_cache.insert(obj_name.to_string(), methods);
            }

            if let Some(methods) = object_cache.get(obj_name)
                && let Some(op_kind) = methods.get(method_name)
            {
                match op_kind.as_str() {
                    "reverse" => ops.push(SigOp::Reverse),
                    "slice" => ops.push(SigOp::Slice(arg_val)),
                    "splice" => ops.push(SigOp::Splice(arg_val)),
                    "swap" => ops.push(SigOp::Swap(arg_val)),
                    _ => {}
                }
            }
        }

        Ok(ops)
    }

    pub fn decipher_signature(&self, sig: &str) -> String {
        if self.sig_ops.is_empty() {
            return sig.to_string();
        }

        let mut chars: Vec<char> = sig.chars().collect();
        for op in &self.sig_ops {
            match op {
                SigOp::Reverse => chars.reverse(),
                SigOp::Slice(n) => {
                    if *n < chars.len() {
                        chars = chars[*n..].to_vec();
                    }
                }
                SigOp::Splice(n) => {
                    if *n < chars.len() {
                        chars.drain(0..*n);
                    }
                }
                SigOp::Swap(pos) => {
                    if !chars.is_empty() {
                        let target = *pos % chars.len();
                        chars.swap(0, target);
                    }
                }
            }
        }

        chars.into_iter().collect()
    }

    pub fn solve_n_param(&self, n: &str, player_js: Option<&str>) -> String {
        if n.is_empty() {
            return n.to_string();
        }

        if let Some(js) = player_js
            && let Some(transformed) = self.execute_n_transform(js, n)
        {
            return transformed;
        }

        n.to_string()
    }

    fn execute_n_transform(&self, js: &str, n: &str) -> Option<String> {
        let cap = N_FUNC_NAME_RE.captures(js)?;
        let func_name = cap
            .get(1)
            .or_else(|| cap.get(3))
            .or_else(|| cap.get(5))?
            .as_str();

        let func_decl = format!(
            r#"(?:var\s+)?{func_name}\s*=\s*function\("#,
            func_name = regex::escape(func_name)
        );
        let decl_re = Regex::new(&func_decl).ok()?;
        let start_pos = decl_re.find(js)?.start();

        let body = extract_balanced_braces(&js[start_pos..])?;

        let mut chars: Vec<char> = n.chars().collect();

        let swap_re = Regex::new(r#"([a-zA-Z0-9$]+)\.reverse\(\)"#).ok()?;
        if swap_re.is_match(&body) {
            chars.reverse();
        }

        let slice_re = Regex::new(r#"([a-zA-Z0-9$]+)\.slice\((\d+)\)"#).ok()?;
        if let Some(scap) = slice_re.captures(&body)
            && let Some(num) = scap.get(2).and_then(|m| m.as_str().parse::<usize>().ok())
            && num < chars.len()
        {
            chars = chars[num..].to_vec();
        }

        let splice_re = Regex::new(r#"([a-zA-Z0-9$]+)\.splice\((\d+),\s*(\d+)\)"#).ok()?;
        if let Some(spcap) = splice_re.captures(&body)
            && let Some(idx) = spcap.get(2).and_then(|m| m.as_str().parse::<usize>().ok())
            && let Some(count) = spcap.get(3).and_then(|m| m.as_str().parse::<usize>().ok())
            && idx < chars.len()
        {
            let end = (idx + count).min(chars.len());
            chars.drain(idx..end);
        }

        if chars.is_empty() {
            Some(n.to_string())
        } else {
            Some(chars.into_iter().collect())
        }
    }
}

/// Extracts content enclosed within balanced curly braces `{ ... }` handling strings & nested blocks
pub fn extract_balanced_braces(js_snippet: &str) -> Option<String> {
    let brace_start = js_snippet.find('{')?;
    let chars: Vec<char> = js_snippet[brace_start..].chars().collect();

    let mut depth = 0;
    let mut in_str = None;
    let mut is_escaped = false;
    let mut end_idx = 0;

    for (i, &ch) in chars.iter().enumerate() {
        if is_escaped {
            is_escaped = false;
            continue;
        }

        if ch == '\\' {
            is_escaped = true;
            continue;
        }

        if let Some(quote) = in_str {
            if ch == quote {
                in_str = None;
            }
        } else {
            match ch {
                '"' | '\'' | '`' => in_str = Some(ch),
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end_idx = i;
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    if depth == 0 && end_idx > 0 {
        let inside: String = chars[1..end_idx].iter().collect();
        Some(inside)
    } else {
        None
    }
}

fn parse_helper_object(js: &str, obj_name: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let obj_decl_pattern = format!(
        r#"(?:var\s+)?{obj_name}\s*=\s*\{{"#,
        obj_name = regex::escape(obj_name)
    );
    let re = match Regex::new(&obj_decl_pattern) {
        Ok(r) => r,
        Err(_) => return map,
    };

    let start_pos = match re.find(js) {
        Some(m) => m.start(),
        None => return map,
    };

    let obj_body = match extract_balanced_braces(&js[start_pos..]) {
        Some(b) => b,
        None => return map,
    };

    let method_re =
        Regex::new(r#"(?s)([a-zA-Z0-9$]+)\s*:\s*function\s*\([^)]*\)\s*\{([^}]+)\}"#).unwrap();
    for cap in method_re.captures_iter(&obj_body) {
        let method_name = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        let method_body = cap.get(2).map(|m| m.as_str()).unwrap_or_default();

        if method_body.contains("reverse") {
            map.insert(method_name.to_string(), "reverse".to_string());
        } else if method_body.contains("splice") {
            map.insert(method_name.to_string(), "splice".to_string());
        } else if method_body.contains("slice") {
            map.insert(method_name.to_string(), "slice".to_string());
        } else if method_body.contains("%")
            || method_body.contains("c=a[0]")
            || method_body.contains("var c=a")
            || method_body.contains("a[b]")
        {
            map.insert(method_name.to_string(), "swap".to_string());
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_balanced_braces() {
        let snippet = r#"function(a){if(a){try{var x="{hello}";}catch(e){}}return a.join("");}"#;
        let body = extract_balanced_braces(snippet).unwrap();
        assert!(body.contains("try"));
        assert!(body.contains("return a.join"));
    }

    #[test]
    fn test_parse_real_player_js_pattern() {
        let fake_js = r#"
            var ab = {
                rq: function(a) { a.reverse() },
                wx: function(a, b) { a.splice(0, b) },
                yz: function(a, b) { var c = a[0]; a[0] = a[b % a.length]; a[b % a.length] = c }
            };
            var cd = function(a) {
                a = a.split("");
                ab.rq(a, 12);
                ab.wx(a, 2);
                ab.yz(a, 5);
                return a.join("");
            };
        "#;

        let mut solver = JsChallengeSolver::new();
        solver.parse_player_js(fake_js).unwrap();
        assert_eq!(solver.sig_ops.len(), 3);
        assert_eq!(solver.sig_ops[0], SigOp::Reverse);
        assert_eq!(solver.sig_ops[1], SigOp::Splice(2));
        assert_eq!(solver.sig_ops[2], SigOp::Swap(5));

        let res = solver.decipher_signature("abcdefghijklmnop");
        assert_ne!(res, "abcdefghijklmnop");
    }
}
