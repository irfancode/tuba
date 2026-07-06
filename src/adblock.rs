/// Simple URL-based ad blocker — G11
/// Uses pattern matching against known ad/tracker domains.

const BLOCKED_DOMAINS: &[&str] = &[
    // Ad networks
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "google-analytics.com",
    "googletagmanager.com",
    "adservice.google.com",
    "pagead2.googlesyndication.com",
    // Tracking / analytics
    "facebook.net",
    "facebook.com/tr",
    "connect.facebook.net",
    "analytics.twitter.com",
    "ads-twitter.com",
    "pixel.quantserve.com",
    "scorecardresearch.com",
    "comscore.net",
    "comscore.com",
    "hotjar.com",
    "mouseflow.com",
    "crazyegg.com",
    // Ad servers
    "adnxs.com",
    "rubiconproject.com",
    "criteo.com",
    "criteo.net",
    "pubmatic.com",
    "openx.net",
    "sharethrough.com",
    "indexww.com",
    "casalemedia.com",
    "contextweb.com",
    // Social widgets / tracking
    "platform.twitter.com",
    "platform.instagram.com",
    "static.xx.fbcdn.net",
    "sdk.scdn.co",
    // Crypto mining
    "coin-hive.com",
    "coinhive.com",
    // Malvertising
    "advertising.com",
    "atdmt.com",
    "media.net",
    "outbrain.com",
    "taboola.com",
    "revcontent.com",
    // Analytics
    "amplitude.com",
    "segment.io",
    "segment.com",
    "mixpanel.com",
    "heap.io",
    "fullstory.com",
    "clicky.com",
    "matomo.org",
    "piwik.org",
    "sentry.io",
    "datadoghq.com",
];

const BLOCKED_PATTERNS: &[&str] = &[
    "/ads/",
    "/ad/",
    "/banners/",
    "/analytics/",
    "/pixel.",
    "utm_source=",
    "utm_medium=",
    "utm_campaign=",
    "fbclid=",
    "gclid=",
    "_ga=",
];

pub struct AdBlocker {
    blocked_domains: Vec<String>,
    blocked_patterns: Vec<String>,
}

impl AdBlocker {
    pub fn new() -> Self {
        Self {
            blocked_domains: BLOCKED_DOMAINS.iter().map(|s| s.to_string()).collect(),
            blocked_patterns: BLOCKED_PATTERNS.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn is_blocked(&self, url: &str) -> bool {
        let lower = url.to_lowercase();

        for domain in &self.blocked_domains {
            if lower.contains(domain) {
                return true;
            }
        }

        for pattern in &self.blocked_patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        false
    }
}
