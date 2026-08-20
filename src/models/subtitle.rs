use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub language_code: String,
    pub name: String,
    pub base_url: String,
    pub is_auto_generated: bool,
}

impl SubtitleTrack {
    pub fn vtt_url(&self) -> String {
        if self.base_url.contains("fmt=") {
            self.base_url.clone()
        } else {
            format!("{}&fmt=vtt", self.base_url)
        }
    }

    pub fn srv3_url(&self) -> String {
        if self.base_url.contains("fmt=") {
            self.base_url.clone()
        } else {
            format!("{}&fmt=srv3", self.base_url)
        }
    }
}

pub struct SubtitleConverter;

impl SubtitleConverter {
    /// Converts YouTube TimedText format 3 XML to WebVTT format
    pub fn timedtext_to_vtt(xml: &str) -> String {
        if xml.starts_with("WEBVTT") {
            return xml.to_string();
        }

        let mut vtt = String::from("WEBVTT\nKind: captions\n\n");
        let re_p =
            regex::Regex::new(r#"<p\s+t="(\d+)"(?:\s+d="(\d+)")?[^>]*>([\s\S]*?)</p>"#).unwrap();
        let re_text = regex::Regex::new(
            r#"<text\s+start="([\d\.]+)"(?:\s+dur="([\d\.]+)")?[^>]*>([\s\S]*?)</text>"#,
        )
        .unwrap();

        let mut matched = false;

        for cap in re_p.captures_iter(xml) {
            matched = true;
            let start_ms: u64 = cap
                .get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let dur_ms: u64 = cap
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(2000);
            let raw_text = cap.get(3).map(|m| m.as_str()).unwrap_or("");

            let text = Self::clean_xml_text(raw_text);
            if text.is_empty() {
                continue;
            }

            let start_ts = Self::format_vtt_timestamp(start_ms);
            let end_ts = Self::format_vtt_timestamp(start_ms + dur_ms);

            vtt.push_str(&format!("{} --> {}\n{}\n\n", start_ts, end_ts, text));
        }

        if !matched {
            for cap in re_text.captures_iter(xml) {
                let start_sec: f64 = cap
                    .get(1)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(0.0);
                let dur_sec: f64 = cap
                    .get(2)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(2.0);
                let raw_text = cap.get(3).map(|m| m.as_str()).unwrap_or("");

                let text = Self::clean_xml_text(raw_text);
                if text.is_empty() {
                    continue;
                }

                let start_ms = (start_sec * 1000.0) as u64;
                let dur_ms = (dur_sec * 1000.0) as u64;

                let start_ts = Self::format_vtt_timestamp(start_ms);
                let end_ts = Self::format_vtt_timestamp(start_ms + dur_ms);

                vtt.push_str(&format!("{} --> {}\n{}\n\n", start_ts, end_ts, text));
            }
        }

        vtt
    }

    /// Converts YouTube TimedText format 3 XML to SRT format
    pub fn timedtext_to_srt(xml: &str) -> String {
        let mut srt = String::new();
        let re_p =
            regex::Regex::new(r#"<p\s+t="(\d+)"(?:\s+d="(\d+)")?[^>]*>([\s\S]*?)</p>"#).unwrap();
        let re_text = regex::Regex::new(
            r#"<text\s+start="([\d\.]+)"(?:\s+dur="([\d\.]+)")?[^>]*>([\s\S]*?)</text>"#,
        )
        .unwrap();

        let mut index = 1;
        let mut matched = false;

        for cap in re_p.captures_iter(xml) {
            matched = true;
            let start_ms: u64 = cap
                .get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let dur_ms: u64 = cap
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(2000);
            let raw_text = cap.get(3).map(|m| m.as_str()).unwrap_or("");

            let text = Self::clean_xml_text(raw_text);
            if text.is_empty() {
                continue;
            }

            let start_ts = Self::format_srt_timestamp(start_ms);
            let end_ts = Self::format_srt_timestamp(start_ms + dur_ms);

            srt.push_str(&format!(
                "{}\n{} --> {}\n{}\n\n",
                index, start_ts, end_ts, text
            ));
            index += 1;
        }

        if !matched {
            for cap in re_text.captures_iter(xml) {
                let start_sec: f64 = cap
                    .get(1)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(0.0);
                let dur_sec: f64 = cap
                    .get(2)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(2.0);
                let raw_text = cap.get(3).map(|m| m.as_str()).unwrap_or("");

                let text = Self::clean_xml_text(raw_text);
                if text.is_empty() {
                    continue;
                }

                let start_ms = (start_sec * 1000.0) as u64;
                let dur_ms = (dur_sec * 1000.0) as u64;

                let start_ts = Self::format_srt_timestamp(start_ms);
                let end_ts = Self::format_srt_timestamp(start_ms + dur_ms);

                srt.push_str(&format!(
                    "{}\n{} --> {}\n{}\n\n",
                    index, start_ts, end_ts, text
                ));
                index += 1;
            }
        }

        srt
    }

    fn clean_xml_text(raw: &str) -> String {
        let mut s = raw.to_string();
        let re_tag = regex::Regex::new(r"<[^>]+>").unwrap();
        s = re_tag.replace_all(&s, "").to_string();

        s = s
            .replace("&amp;", "&")
            .replace("&#39;", "'")
            .replace("&quot;", "\"")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&#x20;", " ")
            .replace("&#160;", " ");

        s.trim().to_string()
    }

    fn format_vtt_timestamp(ms: u64) -> String {
        let hrs = ms / 3_600_000;
        let mins = (ms % 3_600_000) / 60_000;
        let secs = (ms % 60_000) / 1_000;
        let millis = ms % 1_000;
        format!("{:02}:{:02}:{:02}.{:03}", hrs, mins, secs, millis)
    }

    fn format_srt_timestamp(ms: u64) -> String {
        let hrs = ms / 3_600_000;
        let mins = (ms % 3_600_000) / 60_000;
        let secs = (ms % 60_000) / 1_000;
        let millis = ms % 1_000;
        format!("{:02}:{:02}:{:02},{:03}", hrs, mins, secs, millis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timedtext_to_srt() {
        let xml = r#"<timedtext format="3"><body><p t="18640" d="3240">♪ We&#39;re no strangers to love ♪</p></body></timedtext>"#;
        let srt = SubtitleConverter::timedtext_to_srt(xml);
        assert!(srt.contains("1\n00:00:18,640 --> 00:00:21,880\n♪ We're no strangers to love ♪"));
    }

    #[test]
    fn test_timedtext_to_vtt() {
        let xml = r#"<timedtext format="3"><body><p t="18640" d="3240">♪ We&#39;re no strangers to love ♪</p></body></timedtext>"#;
        let vtt = SubtitleConverter::timedtext_to_vtt(xml);
        assert!(vtt.contains("00:00:18.640 --> 00:00:21.880\n♪ We're no strangers to love ♪"));
    }
}
