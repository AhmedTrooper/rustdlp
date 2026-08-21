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

        let func_pat = format!(
            r#"(?s){func_name}\s*=\s*function\s*\(([a-zA-Z0-9$]+)\)\s*\{{(?P<body>[^\}}]+)\}}"#,
            func_name = regex::escape(func_name)
        );
        let func_re = Regex::new(&func_pat).ok()?;
        let body = func_re.captures(js)?.name("body")?.as_str();

        let mut chars: Vec<char> = n.chars().collect();

        let swap_re = Regex::new(r#"([a-zA-Z0-9$]+)\.reverse\(\)"#).ok()?;
        if swap_re.is_match(body) {
            chars.reverse();
        }

        let slice_re = Regex::new(r#"([a-zA-Z0-9$]+)\.slice\((\d+)\)"#).ok()?;
        if let Some(scap) = slice_re.captures(body)
            && let Some(num) = scap.get(2).and_then(|m| m.as_str().parse::<usize>().ok())
            && num < chars.len()
        {
            chars = chars[num..].to_vec();
        }

        let splice_re = Regex::new(r#"([a-zA-Z0-9$]+)\.splice\((\d+),\s*(\d+)\)"#).ok()?;
        if let Some(spcap) = splice_re.captures(body)
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
}
