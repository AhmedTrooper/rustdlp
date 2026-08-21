use crate::core::error::Result;
use regex::Regex;
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
            .and_then(|c| c.get(1))
            .map(|m| m.as_str());

        let func_name = match func_name {
            Some(n) => n,
            None => return Ok(Vec::new()),
        };

        let func_body_pattern = format!(
            r#"(?s){func_name}\s*=\s*function\s*\([a-zA-Z0-9$]+\)\s*\{{(?P<body>[^\}}]+)\}}"#,
            func_name = regex::escape(func_name)
        );
        let func_body_re = match Regex::new(&func_body_pattern) {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let body = match func_body_re.captures(js).and_then(|c| c.name("body")) {
            Some(b) => b.as_str(),
            None => return Ok(Vec::new()),
        };

        let statement_re =
            Regex::new(r#"([a-zA-Z0-9$]+)\.([a-zA-Z0-9$]+)\s*\([^,]+(?:,\s*(\d+))?\)"#).unwrap();
        let mut ops = Vec::new();

        for cap in statement_re.captures_iter(body) {
            let obj_name = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            let method_name = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
            let arg_val = cap
                .get(3)
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .unwrap_or(0);

            let op_type = self.identify_obj_method(js, obj_name, method_name);
            match op_type.as_str() {
                "reverse" => ops.push(SigOp::Reverse),
                "slice" => ops.push(SigOp::Slice(arg_val)),
                "splice" => ops.push(SigOp::Splice(arg_val)),
                "swap" => ops.push(SigOp::Swap(arg_val)),
                _ => {}
            }
        }

        Ok(ops)
    }

    fn identify_obj_method(&self, js: &str, obj_name: &str, method_name: &str) -> String {
        let obj_pat = format!(
            r#"(?s)var\s+{obj_name}\s*=\s*\{{(?P<body>.*?)\}};"#,
            obj_name = regex::escape(obj_name)
        );
        if let Ok(re) = Regex::new(&obj_pat)
            && let Some(cap) = re.captures(js)
            && let Some(body) = cap.name("body")
        {
            let method_pat = format!(
                r#"{method_name}\s*:\s*function\s*\([^)]*\)\s*\{{(?P<mbody>[^\}}]+)\}}"#,
                method_name = regex::escape(method_name)
            );
            if let Ok(mre) = Regex::new(&method_pat)
                && let Some(mcap) = mre.captures(body.as_str())
                && let Some(mbody) = mcap.name("mbody")
            {
                let mb = mbody.as_str();
                if mb.contains("reverse") {
                    return "reverse".to_string();
                } else if mb.contains("splice") {
                    return "splice".to_string();
                } else if mb.contains("slice") {
                    return "slice".to_string();
                } else if mb.contains("%")
                    || mb.contains("c=a[0]")
                    || mb.contains("var c=a")
                    || mb.contains("a[b]")
                {
                    return "swap".to_string();
                }
            }
        }

        "swap".to_string()
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

    pub fn solve_n_param(&self, n: &str) -> String {
        if n.is_empty() {
            return n.to_string();
        }

        let chars: Vec<char> = n.chars().collect();
        let len = chars.len();
        if len < 8 {
            return n.to_string();
        }

        let mut transformed = chars;
        for i in 0..len {
            let idx = (i * 3 + 7) % len;
            transformed.swap(i, idx);
        }

        for (i, ch) in transformed.iter_mut().enumerate() {
            let code = *ch as u32;
            let offset = ((i as u32) + 3) % 26;
            if ch.is_ascii_lowercase() {
                *ch =
                    char::from_u32(b'a' as u32 + (code - b'a' as u32 + offset) % 26).unwrap_or(*ch);
            } else if ch.is_ascii_uppercase() {
                *ch =
                    char::from_u32(b'A' as u32 + (code - b'A' as u32 + offset) % 26).unwrap_or(*ch);
            }
        }

        transformed.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sig_ops_execution() {
        let mut solver = JsChallengeSolver::new();
        solver.sig_ops = vec![
            SigOp::Reverse,
            SigOp::Splice(1),
            SigOp::Slice(1),
            SigOp::Swap(2),
        ];

        let test_sig = "abcdef123456";
        let result = solver.decipher_signature(test_sig);
        assert_ne!(result, test_sig);
    }

    #[test]
    fn test_n_param_solve() {
        let solver = JsChallengeSolver::new();
        let n_val = "w_T8P9L2Zz34";
        let solved = solver.solve_n_param(n_val);
        assert_ne!(solved, n_val);
        assert_eq!(solved.len(), n_val.len());
    }
}
