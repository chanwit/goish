// Port of go1.25.5 src/hash/fnv/fnv_test.go — FNV-1 and FNV-1a 32/64 bit.
//
// Skipped: FNV-128 (not implemented in goish v0.11).

#![allow(non_snake_case)]
use goish::prelude::*;

// Golden vectors from Go's src/hash/fnv/fnv_test.go.
struct GoldenFnv32 { out: u32, inp: &'static [u8] }
struct GoldenFnv64 { out: u64, inp: &'static [u8] }

fn golden32() -> Vec<GoldenFnv32> {
    vec![
        GoldenFnv32 { out: 0x811c9dc5, inp: b"" },
        GoldenFnv32 { out: 0x050c5d7e, inp: b"a" },
        GoldenFnv32 { out: 0x70772d38, inp: b"ab" },
        GoldenFnv32 { out: 0x439c2f4b, inp: b"abc" },
    ]
}

fn golden32a() -> Vec<GoldenFnv32> {
    vec![
        GoldenFnv32 { out: 0x811c9dc5, inp: b"" },
        GoldenFnv32 { out: 0xe40c292c, inp: b"a" },
        GoldenFnv32 { out: 0x4d2505ca, inp: b"ab" },
        GoldenFnv32 { out: 0x1a47e90b, inp: b"abc" },
    ]
}

fn golden64() -> Vec<GoldenFnv64> {
    vec![
        GoldenFnv64 { out: 0xcbf29ce484222325, inp: b"" },
        GoldenFnv64 { out: 0xaf63bd4c8601b7be, inp: b"a" },
        GoldenFnv64 { out: 0x08326707b4eb37b8, inp: b"ab" },
        GoldenFnv64 { out: 0xd8dcca186bafadcb, inp: b"abc" },
    ]
}

fn golden64a() -> Vec<GoldenFnv64> {
    vec![
        GoldenFnv64 { out: 0xcbf29ce484222325, inp: b"" },
        GoldenFnv64 { out: 0xaf63dc4c8601ec8c, inp: b"a" },
        GoldenFnv64 { out: 0x089c4407b545986a, inp: b"ab" },
        GoldenFnv64 { out: 0xe71fa2190541574b, inp: b"abc" },
    ]
}

test!{ fn TestGolden32(t) {
    let __golden32 = golden32();
    for (_, g) in range!(__golden32) {
        let mut h = hash::fnv::New32();
        h.Write(g.inp);
        let got = h.Sum32();
        if got != g.out {
            t.Errorf(Sprintf!("New32().Write(%q).Sum32() = %x, want %x",
                bytes::String(g.inp),
                got as i64, g.out as i64));
        }
    }
}}

test!{ fn TestGolden32a(t) {
    let __golden32a = golden32a();
    for (_, g) in range!(__golden32a) {
        let mut h = hash::fnv::New32a();
        h.Write(g.inp);
        let got = h.Sum32();
        if got != g.out {
            t.Errorf(Sprintf!("New32a().Write(%q).Sum32() = %x, want %x",
                bytes::String(g.inp),
                got as i64, g.out as i64));
        }
    }
}}

test!{ fn TestGolden64(t) {
    let __golden64 = golden64();
    for (_, g) in range!(__golden64) {
        let mut h = hash::fnv::New64();
        h.Write(g.inp);
        let got = h.Sum64();
        if got != g.out {
            t.Errorf(Sprintf!("New64().Write(%q).Sum64() mismatch for %q",
                bytes::String(g.inp),
                bytes::String(g.inp)));
        }
    }
}}

test!{ fn TestGolden64a(t) {
    let __golden64a = golden64a();
    for (_, g) in range!(__golden64a) {
        let mut h = hash::fnv::New64a();
        h.Write(g.inp);
        let got = h.Sum64();
        if got != g.out {
            t.Errorf(Sprintf!("New64a().Write(%q).Sum64() mismatch for %q",
                bytes::String(g.inp),
                bytes::String(g.inp)));
        }
    }
}}

test!{ fn TestReset(t) {
    let mut h = hash::fnv::New32a();
    h.Write(b"garbage");
    h.Reset();
    h.Write(b"abc");
    if h.Sum32() != 0x1a47e90b {
        t.Errorf(Sprintf!("reset failed: %x", h.Sum32() as i64));
    }
}}

test!{ fn TestStreaming(t) {
    // Split-write should match one-shot.
    let inp = b"abcdefghij";
    let want = {
        let mut h = hash::fnv::New32a();
        h.Write(inp);
        h.Sum32()
    };
    let mut h = hash::fnv::New32a();
    h.Write(&inp[..4]);
    h.Write(&inp[4..]);
    if h.Sum32() != want {
        t.Errorf(Sprintf!("streaming 32a mismatch"));
    }
}}
