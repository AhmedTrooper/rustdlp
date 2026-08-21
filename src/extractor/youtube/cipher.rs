use crate::core::error::{DlpError, Result};
use crate::extractor::youtube::jsc::JsChallengeSolver;
use std::collections::HashMap;
use url::Url;
use url::form_urlencoded;

pub struct CipherHelper;

impl CipherHelper {
    /// Parses a `signatureCipher` query string, applies JS deciphering if solver provided, and returns the stream URL
    pub fn parse_signature_cipher(
        cipher_str: &str,
        solver: Option<&JsChallengeSolver>,
    ) -> Result<String> {
        let mut params = HashMap::new();
        for (k, v) in form_urlencoded::parse(cipher_str.as_bytes()) {
            params.insert(k.into_owned(), v.into_owned());
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
