// crypto: namespace for cryptographic primitives.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   "crypto/md5"                        goish::crypto::md5
//   "crypto/sha1"                       goish::crypto::sha1
//   "crypto/sha256"                     goish::crypto::sha256
//
// Go's `crypto` package itself defines the `Hash` enum + PublicKey interfaces;
// those land in v0.5+ alongside any AEAD support. For now this is a namespace
// for the digest sub-packages.

pub mod md5;
pub mod sha1;
pub mod sha256;
