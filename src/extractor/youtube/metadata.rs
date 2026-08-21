use crate::models::subtitle::SubtitleTrack;
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct Chapter {
    pub title: String,
    pub start_time: f64,
    pub end_time: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct StoryboardSpec {
    pub url_template: String,
    pub thumbnail_width: u32,
    pub thumbnail_height: u32,
    pub columns: u32,
    pub rows: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct HeatmapMarker {
    pub start_time: f64,
    pub end_time: Option<f64>,
    pub intensity: f64, // Normalized 0.0..1.0 float matching yt-dlp _video.py contract
}

#[derive(Debug, Clone, Default)]
pub struct YoutubeRichMetadata {
    pub chapters: Vec<Chapter>,
    pub storyboards: Vec<StoryboardSpec>,
    pub heatmaps: Vec<HeatmapMarker>,
    pub subtitles: Vec<SubtitleTrack>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub like_count: Option<u64>,
    pub subscriber_count: Option<String>,
    pub is_verified: bool,
    pub is_live: bool,
}

pub struct YoutubeMetadataParser;

impl YoutubeMetadataParser {
    pub fn parse(val: &Value, html: Option<&str>) -> YoutubeRichMetadata {
        let mut meta = YoutubeRichMetadata::default();

        // 1. Parse Subtitles (Manual & ASR TimedText)
        if let Some(captions) = val
            .pointer("/captions/playerCaptionsTracklistRenderer/captionTracks")
            .and_then(|v| v.as_array())
        {
            for track in captions {
                if let Some(base_url) = track.get("baseUrl").and_then(|v| v.as_str()) {
                    let lang_code = track
                        .get("languageCode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("en")
                        .to_string();
                    let name = track
                        .pointer("/name/simpleText")
                        .or_else(|| track.pointer("/name/runs/0/text"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&lang_code)
                        .to_string();
                    let kind = track
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    let is_asr = kind == "asr";

                    meta.subtitles.push(SubtitleTrack {
                        language_code: lang_code,
                        name,
                        base_url: base_url.to_string(),
                        is_auto_generated: is_asr,
                    });
                }
            }
        }

        // 2. Parse Chapters
        if let Some(markers) = val
            .pointer("/playerOverlays/playerOverlayRenderer/decoratedPlayerBarRenderer/decoratedPlayerBarRenderer/playerBar/multiMarkersPlayerBarRenderer/markersMap")
            .and_then(|v| v.as_array())
        {
            for marker_group in markers {
                if let Some(chapter_list) = marker_group.pointer("/value/chapters").and_then(|v| v.as_array()) {
                    for ch in chapter_list {
                        let renderer = ch.get("chapterRenderer").unwrap_or(ch);
                        let title = renderer
                            .pointer("/title/simpleText")
                            .or_else(|| renderer.pointer("/title/runs/0/text"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Chapter")
                            .to_string();
                        let start_ms = renderer
                            .get("timeRangeStartMillis")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        meta.chapters.push(Chapter {
                            title,
                            start_time: (start_ms as f64) / 1000.0,
                            end_time: None,
                        });
                    }
                }
            }
        }

        // Fallback: description timestamp chapters
        if meta.chapters.is_empty()
            && let Some(desc) = val
                .pointer("/videoDetails/shortDescription")
                .and_then(|v| v.as_str())
        {
            let ts_re =
                Regex::new(r#"(?m)^\s*(?:(\d{1,2}):)?(\d{2}):(\d{2})\s+[-–—]?\s*(.+)$"#).unwrap();
            for cap in ts_re.captures_iter(desc) {
                let hrs = cap
                    .get(1)
                    .map(|m| m.as_str().parse::<f64>().unwrap_or(0.0))
                    .unwrap_or(0.0);
                let mins = cap
                    .get(2)
                    .and_then(|m| m.as_str().parse::<f64>().ok())
                    .unwrap_or(0.0);
                let secs = cap
                    .get(3)
                    .and_then(|m| m.as_str().parse::<f64>().ok())
                    .unwrap_or(0.0);
                let title = cap
                    .get(4)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();

                let total_secs = hrs * 3600.0 + mins * 60.0 + secs;
                meta.chapters.push(Chapter {
                    title,
                    start_time: total_secs,
                    end_time: None,
                });
            }
        }

        // 3. Parse Storyboards
        if let Some(spec) = val
            .pointer("/storyboards/playerStoryboardSpecRenderer/spec")
            .and_then(|v| v.as_str())
        {
            for part in spec.split('|') {
                let segments: Vec<&str> = part.split('#').collect();
                if segments.len() >= 5 {
                    meta.storyboards.push(StoryboardSpec {
                        url_template: segments[0].to_string(),
                        thumbnail_width: segments[1].parse().unwrap_or(160),
                        thumbnail_height: segments[2].parse().unwrap_or(90),
                        columns: segments[3].parse().unwrap_or(5),
                        rows: segments[4].parse().unwrap_or(5),
                        count: segments.get(5).and_then(|s| s.parse().ok()).unwrap_or(25),
                    });
                }
            }
        }

        // 4. Parse Heatmaps (_video.py:2364)
        if let Some(markers_map) = val
            .pointer("/frameworkUpdates/entityBatchUpdate/mutations")
            .and_then(|v| v.as_array())
        {
            for mutation in markers_map {
                if let Some(marker_type) = mutation
                    .pointer("/payload/macroMarkersListEntity/markersList/markerType")
                    .and_then(|v| v.as_str())
                    && marker_type != "MARKER_TYPE_HEATMAP"
                {
                    continue;
                }
                if let Some(heat_markers) = mutation
                    .pointer("/payload/macroMarkersListEntity/markersList/markers")
                    .and_then(|v| v.as_array())
                {
                    for marker in heat_markers {
                        if let (Some(start_ms), Some(intensity)) = (
                            marker
                                .get("startMillis")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<f64>().ok()),
                            marker
                                .get("intensityScoreNormalized")
                                .and_then(|v| v.as_f64()),
                        ) {
                            let duration_ms = marker
                                .get("durationMillis")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<f64>().ok());
                            let start_time = start_ms / 1000.0;
                            let end_time = duration_ms.map(|d| (start_ms + d) / 1000.0);
                            meta.heatmaps.push(HeatmapMarker {
                                start_time,
                                end_time,
                                intensity,
                            });
                        }
                    }
                }
            }
        }

        // 5. Subscriber Count & Verification Badges
        meta.subscriber_count = val.pointer("/contents/twoColumnWatchNextResults/results/results/contents/1/videoSecondaryInfoRenderer/owner/videoOwnerRenderer/subscriberCountText/simpleText")
            .or_else(|| val.pointer("/contents/twoColumnWatchNextResults/results/results/contents/1/videoSecondaryInfoRenderer/owner/videoOwnerRenderer/subscriberCountText/runs/0/text"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(badges) = val.pointer("/contents/twoColumnWatchNextResults/results/results/contents/1/videoSecondaryInfoRenderer/owner/videoOwnerRenderer/badges").and_then(|v| v.as_array()) {
            for b in badges {
                if let Some(style) = b.pointer("/metadataBadgeRenderer/style").and_then(|v| v.as_str())
                    && style.contains("VERIFIED")
                {
                    meta.is_verified = true;
                    break;
                }
            }
        }

        // 6. Like Count (from JSON)
        meta.like_count = val.pointer("/videoDetails/likeCount")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .or_else(|| {
                val.pointer("/contents/twoColumnWatchNextResults/results/results/contents/0/videoPrimaryInfoRenderer/videoActions/menuRenderer/topLevelButtons/0/segmentedLikeDislikeButtonViewModel/likeButtonViewModel/likeButtonViewModel/toggleButtonViewModel/toggleButtonViewModel/defaultButtonViewModel/buttonViewModel/title")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.replace(',', "").parse::<u64>().ok())
            });

        // 7. Tags & Categories
        if let Some(keywords) = val
            .pointer("/videoDetails/keywords")
            .and_then(|v| v.as_array())
        {
            meta.tags = keywords
                .iter()
                .filter_map(|k| k.as_str())
                .map(|s| s.to_string())
                .collect();
        }

        if let Some(cat) = val
            .pointer("/microformat/playerMicroformatRenderer/category")
            .and_then(|v| v.as_str())
        {
            meta.categories.push(cat.to_string());
        }

        meta.is_live = val
            .pointer("/videoDetails/isLive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || val
                .pointer("/videoDetails/isLiveContent")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

        // Fallback: HTML markers
        if meta.like_count.is_none()
            && let Some(page) = html
        {
            let likes_re = Regex::new(r#""label":"([0-9,]+)\s+likes"#).unwrap();
            if let Some(cap) = likes_re.captures(page)
                && let Some(m) = cap.get(1)
            {
                let clean = m.as_str().replace(',', "");
                meta.like_count = clean.parse::<u64>().ok();
            }
        }

        meta
    }
}
