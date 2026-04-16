// mime: Go's mime package — MIME type lookups by extension.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   mime.TypeByExtension(".html")       mime::TypeByExtension(".html")
//   mime.ExtensionsByType("text/html")  mime::ExtensionsByType("text/html")
//   mime.AddExtensionType(".x", "…")    mime::AddExtensionType(".x", "…")
//
// Covers the common web / text / image / audio / video / archive types. Go's
// table is much larger; callers needing something obscure can register with
// AddExtensionType.

pub mod multipart;

use crate::errors::{error, nil, New};
use crate::types::{slice, string};
use std::sync::{OnceLock, RwLock};

fn table() -> &'static RwLock<Vec<(string, string)>> {
    static T: OnceLock<RwLock<Vec<(string, string)>>> = OnceLock::new();
    T.get_or_init(|| {
        let base: &[(&str, &str)] = &[
            (".html", "text/html; charset=utf-8"),
            (".htm",  "text/html; charset=utf-8"),
            (".css",  "text/css; charset=utf-8"),
            (".js",   "text/javascript; charset=utf-8"),
            (".mjs",  "text/javascript; charset=utf-8"),
            (".json", "application/json"),
            (".xml",  "text/xml; charset=utf-8"),
            (".txt",  "text/plain; charset=utf-8"),
            (".md",   "text/markdown; charset=utf-8"),
            (".csv",  "text/csv; charset=utf-8"),
            (".pdf",  "application/pdf"),
            (".zip",  "application/zip"),
            (".gz",   "application/gzip"),
            (".tar",  "application/x-tar"),
            (".bz2",  "application/x-bzip2"),
            (".7z",   "application/x-7z-compressed"),
            (".wasm", "application/wasm"),
            (".ico",  "image/x-icon"),
            (".png",  "image/png"),
            (".jpg",  "image/jpeg"),
            (".jpeg", "image/jpeg"),
            (".gif",  "image/gif"),
            (".webp", "image/webp"),
            (".svg",  "image/svg+xml"),
            (".bmp",  "image/bmp"),
            (".avif", "image/avif"),
            (".mp3",  "audio/mpeg"),
            (".wav",  "audio/wav"),
            (".ogg",  "audio/ogg"),
            (".flac", "audio/flac"),
            (".mp4",  "video/mp4"),
            (".webm", "video/webm"),
            (".mov",  "video/quicktime"),
            (".ttf",  "font/ttf"),
            (".otf",  "font/otf"),
            (".woff", "font/woff"),
            (".woff2","font/woff2"),
        ];
        RwLock::new(base.iter().map(|(e, t)| (string::from(*e), string::from(*t))).collect())
    })
}

#[allow(non_snake_case)]
pub fn TypeByExtension(ext: impl AsRef<str>) -> string {
    let ext = ext.as_ref().to_ascii_lowercase();
    let ext = if ext.starts_with('.') { ext } else { format!(".{}", ext) };
    let tab = table().read().unwrap();
    for (e, t) in tab.iter() {
        if e.as_str() == ext.as_str() { return t.clone(); }
    }
    "".into()
}

#[allow(non_snake_case)]
pub fn ExtensionsByType(type_: impl AsRef<str>) -> (slice<string>, error) {
    let t = type_.as_ref();
    // Strip any parameters for matching.
    let base = t.split(';').next().unwrap_or(t).trim().to_ascii_lowercase();
    let tab = table().read().unwrap();
    let mut out: slice<string> = slice::new();
    for (e, v) in tab.iter() {
        let vb = v.split(';').next().unwrap_or(v).trim().to_ascii_lowercase();
        if vb == base {
            out.push(e.clone());
        }
    }
    out.sort();
    (out, nil)
}

#[allow(non_snake_case)]
pub fn AddExtensionType(ext: impl AsRef<str>, typ: impl AsRef<str>) -> error {
    let ext = ext.as_ref();
    if !ext.starts_with('.') {
        return New(&format!("mime: extension {:?} must start with a dot", ext));
    }
    let mut tab = table().write().unwrap();
    if let Some(entry) = tab.iter_mut().find(|(e, _)| e.as_str() == ext) {
        entry.1 = typ.as_ref().into();
    } else {
        tab.push((ext.to_ascii_lowercase().into(), typ.as_ref().into()));
    }
    nil
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_extensions() {
        assert!(TypeByExtension(".html").starts_with("text/html"));
        assert_eq!(TypeByExtension(".png"), "image/png");
        assert!(TypeByExtension(".json").starts_with("application/json"));
        assert!(TypeByExtension(".woff2").starts_with("font/woff2"));
    }

    #[test]
    fn extension_without_dot() {
        assert_eq!(TypeByExtension("png"), "image/png");
    }

    #[test]
    fn unknown_returns_empty() {
        assert_eq!(TypeByExtension(".nosuch"), "");
    }

    #[test]
    fn extensions_by_type_reverse_lookup() {
        let (exts, err) = ExtensionsByType("image/jpeg");
        assert_eq!(err, nil);
        assert!(exts.contains(&".jpg".into()));
        assert!(exts.contains(&".jpeg".into()));
    }

    #[test]
    fn add_custom_extension() {
        let err = AddExtensionType(".myext", "application/x-my");
        assert_eq!(err, nil);
        assert_eq!(TypeByExtension(".myext"), "application/x-my");
    }

    #[test]
    fn add_without_dot_returns_error() {
        let err = AddExtensionType("bad", "application/x-bad");
        assert!(err != nil);
    }
}
