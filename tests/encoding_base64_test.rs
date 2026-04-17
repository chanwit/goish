// Port of go1.25.5 src/encoding/base64/base64_test.go — pair-based
// encode/decode round-trips across Std / URL / RawStd / RawURL encodings.
//
// Skipped: NewEncoding-with-WithPadding (funnyEncoding), Strict(), and
// Reader/Writer streaming (not in goish v0.11 surface yet).

#![allow(non_snake_case)]
use goish::prelude::*;

struct Pair {
    decoded: &'static [u8],
    encoded: &'static str,
}

fn pairs() -> Vec<Pair> {
    vec![
        // RFC 3548
        Pair { decoded: b"\x14\xfb\x9c\x03\xd9\x7e", encoded: "FPucA9l+" },
        Pair { decoded: b"\x14\xfb\x9c\x03\xd9",    encoded: "FPucA9k=" },
        Pair { decoded: b"\x14\xfb\x9c\x03",         encoded: "FPucAw==" },
        // RFC 4648
        Pair { decoded: b"",        encoded: "" },
        Pair { decoded: b"f",       encoded: "Zg==" },
        Pair { decoded: b"fo",      encoded: "Zm8=" },
        Pair { decoded: b"foo",     encoded: "Zm9v" },
        Pair { decoded: b"foob",    encoded: "Zm9vYg==" },
        Pair { decoded: b"fooba",   encoded: "Zm9vYmE=" },
        Pair { decoded: b"foobar",  encoded: "Zm9vYmFy" },
        // Wikipedia
        Pair { decoded: b"sure.",    encoded: "c3VyZS4=" },
        Pair { decoded: b"sure",     encoded: "c3VyZQ==" },
        Pair { decoded: b"sur",      encoded: "c3Vy" },
        Pair { decoded: b"su",       encoded: "c3U=" },
        Pair { decoded: b"leasure.", encoded: "bGVhc3VyZS4=" },
        Pair { decoded: b"easure.",  encoded: "ZWFzdXJlLg==" },
        Pair { decoded: b"asure.",   encoded: "YXN1cmUu" },
    ]
}

fn url_ref(s: &str) -> String {
    s.replace('+', "-").replace('/', "_")
}

fn raw_ref(s: &str) -> String {
    s.trim_end_matches('=').to_string()
}

fn raw_url_ref(s: &str) -> String {
    raw_ref(&url_ref(s))
}

test!{ fn TestEncodeStd(t) {
    let __pairs = pairs();
    for (_, p) in range!(__pairs) {
        let got = encoding::base64::StdEncoding.EncodeToString(p.decoded);
        if got != p.encoded {
            t.Errorf(Sprintf!("StdEncoding.EncodeToString(%q) = %q, want %q",
                String::from_utf8_lossy(p.decoded).to_string(), got, p.encoded));
        }
    }
}}

test!{ fn TestDecodeStd(t) {
    let __pairs = pairs();
    for (_, p) in range!(__pairs) {
        let (got, err) = encoding::base64::StdEncoding.DecodeString(p.encoded);
        if err != nil {
            t.Errorf(Sprintf!("StdEncoding.DecodeString(%q) error = %s", p.encoded, err));
            continue;
        }
        if got != p.decoded {
            t.Errorf(Sprintf!("StdEncoding.DecodeString(%q) = %v, want %v",
                p.encoded, got.len(), p.decoded.len()));
        }
    }
}}

test!{ fn TestEncodeURL(t) {
    let __pairs = pairs();
    for (_, p) in range!(__pairs) {
        let want = url_ref(p.encoded);
        let got = encoding::base64::URLEncoding.EncodeToString(p.decoded);
        if got != want {
            t.Errorf(Sprintf!("URLEncoding.EncodeToString = %q, want %q", got, want));
        }
    }
}}

test!{ fn TestDecodeURL(t) {
    let __pairs = pairs();
    for (_, p) in range!(__pairs) {
        let encoded = url_ref(p.encoded);
        let (got, err) = encoding::base64::URLEncoding.DecodeString(&encoded);
        if err != nil {
            t.Errorf(Sprintf!("URLEncoding.DecodeString(%q) error = %s", encoded, err));
            continue;
        }
        if got != p.decoded {
            t.Errorf(Sprintf!("URLEncoding.DecodeString mismatch for %q", encoded));
        }
    }
}}

test!{ fn TestEncodeRawStd(t) {
    let __pairs = pairs();
    for (_, p) in range!(__pairs) {
        let want = raw_ref(p.encoded);
        let got = encoding::base64::RawStdEncoding.EncodeToString(p.decoded);
        if got != want {
            t.Errorf(Sprintf!("RawStdEncoding.EncodeToString = %q, want %q", got, want));
        }
    }
}}

test!{ fn TestDecodeRawStd(t) {
    let __pairs = pairs();
    for (_, p) in range!(__pairs) {
        let encoded = raw_ref(p.encoded);
        let (got, err) = encoding::base64::RawStdEncoding.DecodeString(&encoded);
        if err != nil {
            t.Errorf(Sprintf!("RawStdEncoding.DecodeString(%q) error = %s", encoded, err));
            continue;
        }
        if got != p.decoded {
            t.Errorf(Sprintf!("RawStdEncoding.DecodeString mismatch for %q", encoded));
        }
    }
}}

test!{ fn TestEncodeRawURL(t) {
    let __pairs = pairs();
    for (_, p) in range!(__pairs) {
        let want = raw_url_ref(p.encoded);
        let got = encoding::base64::RawURLEncoding.EncodeToString(p.decoded);
        if got != want {
            t.Errorf(Sprintf!("RawURLEncoding.EncodeToString = %q, want %q", got, want));
        }
    }
}}

test!{ fn TestDecodeRawURL(t) {
    let __pairs = pairs();
    for (_, p) in range!(__pairs) {
        let encoded = raw_url_ref(p.encoded);
        let (got, err) = encoding::base64::RawURLEncoding.DecodeString(&encoded);
        if err != nil {
            t.Errorf(Sprintf!("RawURLEncoding.DecodeString(%q) error = %s", encoded, err));
            continue;
        }
        if got != p.decoded {
            t.Errorf(Sprintf!("RawURLEncoding.DecodeString mismatch for %q", encoded));
        }
    }
}}

test!{ fn TestRoundTrip(t) {
    // Fuzz-like round-trip: bytes 0..255 survive encode/decode intact.
    let src: Vec<u8> = (0u8..=255u8).collect();
    let enc = encoding::base64::StdEncoding.EncodeToString(&src);
    let (dec, err) = encoding::base64::StdEncoding.DecodeString(&enc);
    if err != nil {
        t.Errorf(Sprintf!("round-trip DecodeString error: %s", err));
    }
    if dec != src {
        t.Errorf(Sprintf!("round-trip mismatch: len %d vs %d", dec.len(), src.len()));
    }
}}
