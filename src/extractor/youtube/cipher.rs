use crate::core::error::{DlpError, Result};
use crate::extractor::youtube::jsc::JsChallengeSolver;
use std::collections::HashMap;
use url::Url;

pub struct CipherHelper;

impl CipherHelper {
    /// Parses a `signatureCipher` query string, applies JS deciphering if solver provided, and returns the stream URL
    pub fn parse_signature_cipher(
        cipher_str: &str,
        solver: Option<&JsChallengeSolver>,
    ) -> Result<String> {
        let mut params = HashMap::new();
        for pair in cipher_str.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                let decoded_key = urlencoding::decode(k).unwrap_or(std::borrow::Cow::Borrowed(k));
                let decoded_val = urlencoding::decode(v).unwrap_or(std::borrow::Cow::Borrowed(v));
                params.insert(decoded_key.to_string(), decoded_val.to_string());
            }
        }

        let base_url = params.get("url").ok_or_else(|| {
            DlpError::CipherError("signatureCipher missing 'url' parameter".into())
        })?;

        let sig = params.get("s");
        let sp = params.get("sp").map(|s| s.as_str()).unwrap_or("sig");

        if let Some(s) = sig {
            let deciphered_sig = if let Some(sol) = solver {
                sol.decipher_signature(s)
            } else {
                s.to_string()
            };

            let mut url = Url::parse(base_url)
                .map_err(|e| DlpError::CipherError(format!("Invalid URL in cipher: {}", e)))?;
            url.query_pairs_mut().append_pair(sp, &deciphered_sig);
            Ok(url.to_string())
        } else {
            Ok(base_url.to_string())
        }
    }
}

mod urlencoding {
    use std::borrow::Cow;

    pub fn decode(s: &str) -> Option<Cow<'_, str>> {
        let mut res = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '%' {
                let h1 = chars.next()?;
                let h2 = chars.next()?;
                let hex_str = format!("{}{}", h1, h2);
                let byte = u8::from_str_radix(&hex_str, 16).ok()?;
                res.push(byte as char);
            } else if c == '+' {
                res.push(' ');
            } else {
                res.push(c);
            }
        }
        Some(Cow::Owned(res))
    }
}
