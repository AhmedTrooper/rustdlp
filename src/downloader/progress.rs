use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;

#[derive(Clone)]
pub struct DownloadProgressBar {
    pb: Arc<ProgressBar>,
}

impl DownloadProgressBar {
    pub fn new(total_bytes: Option<u64>, prefix: &str) -> Self {
        let pb = if let Some(total) = total_bytes {
            let pb = ProgressBar::new(total);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(&format!(
                        "{{prefix:.bold}} [download] {{percent:>3}}% of {{total_bytes}} at {{binary_bytes_per_sec}} ETA {{eta}}"
                    ))
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("━╸─"),
            );
            pb
        } else {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template(&format!("{{prefix:.bold}} [download] {{bytes}} at {{binary_bytes_per_sec}}"))
                    .unwrap_or_else(|_| ProgressStyle::default_spinner()),
            );
            pb
        };

        pb.set_prefix(prefix.to_string());
        Self { pb: Arc::new(pb) }
    }

    pub fn inc(&self, delta: u64) {
        self.pb.inc(delta);
    }

    pub fn finish_with_message(&self, msg: &str) {
        self.pb.finish_with_message(msg.to_string());
    }

    pub fn finish_and_clear(&self) {
        self.pb.finish_and_clear();
    }
}
