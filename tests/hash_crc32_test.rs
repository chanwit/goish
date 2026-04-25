// Port of go1.25.5 src/hash/crc32/crc32_test.go — IEEE golden vectors.
//
// Skipped: Castagnoli tests (table not pre-generated in goish),
// Koopman tests (same), SSE-specific benchmarks.

#![allow(non_snake_case)]
use goish::prelude::*;

struct Golden {
    ieee: u32,
    inp: &'static str,
}

fn golden() -> slice<Golden> {
    // From Go's src/hash/crc32/crc32_test.go.
    vec![
        Golden { ieee: 0x0,        inp: "" },
        Golden { ieee: 0xe8b7be43, inp: "a" },
        Golden { ieee: 0x9e83486d, inp: "ab" },
        Golden { ieee: 0x352441c2, inp: "abc" },
        Golden { ieee: 0xed82cd11, inp: "abcd" },
        Golden { ieee: 0x8587d865, inp: "abcde" },
        Golden { ieee: 0x4b8e39ef, inp: "abcdef" },
        Golden { ieee: 0x312a6aa6, inp: "abcdefg" },
        Golden { ieee: 0xaeef2a50, inp: "abcdefgh" },
        Golden { ieee: 0x8da988af, inp: "abcdefghi" },
        Golden { ieee: 0x3981703a, inp: "abcdefghij" },
        Golden { ieee: 0x6b9cdfe7, inp: "Discard medicine more than two years old." },
        Golden { ieee: 0xc90ef73f, inp: "He who has a shady past knows that nice guys finish last." },
        Golden { ieee: 0xb902341f, inp: "I wouldn't marry him with a ten foot pole." },
        Golden { ieee: 0x042080e8, inp: "Free! Free!/A trip/to Mars/for 900/empty jars/Burma Shave" },
        Golden { ieee: 0x154c6d11, inp: "The days of the digital watch are numbered.  -Tom Stoppard" },
        Golden { ieee: 0x4c418325, inp: "Nepal premier won't resign." },
        Golden { ieee: 0x33955150, inp: "For every action there is an equal and opposite government program." },
        Golden { ieee: 0x26216a4b, inp: "His money is twice tainted: 'taint yours and 'taint mine." },
        Golden { ieee: 0x1abbe45e, inp: "There is no reason for any individual to have a computer in their home. -Ken Olsen, 1977" },
        Golden { ieee: 0xc89a94f7, inp: "It's a tiny change to the code and not completely disgusting. - Bob Manchek" },
        Golden { ieee: 0xab3abe14, inp: "size:  a.out:  bad magic" },
        Golden { ieee: 0xbab102b6, inp: "The major problem is with sendmail.  -Mark Horton" },
        Golden { ieee: 0x999149d7, inp: "Give me a rock, paper and scissors and I will move the world.  CCFestoon" },
        Golden { ieee: 0x6d52a33c, inp: "If the enemy is within range, then so are you." },
        Golden { ieee: 0x90631e8d, inp: "It's well we cannot hear the screams/That we create in others' dreams." },
        Golden { ieee: 0x78309130, inp: "You remind me of a TV show, but that's all right: I watch it anyway." },
        Golden { ieee: 0x7d0a377f, inp: "C is as portable as Stonehedge!!" },
        Golden { ieee: 0x8c79fd79, inp: "Even if I could be Shakespeare, I think I should still choose to be Faraday. - A. Huxley" },
        Golden { ieee: 0xa20b7167, inp: "The fugacity of a constituent in a mixture of gases at a given temperature is proportional to its mole fraction.  Lewis-Randall Rule" },
        Golden { ieee: 0x8e0bb443, inp: "How can you write a big system without C++?  -Paul Glick" },
    ].into()
}

test!{ fn TestGolden(t) {
    let __golden = golden();
    for (_, g) in range!(__golden) {
        let got = hash::crc32::ChecksumIEEE(g.inp.as_bytes());
        if got != g.ieee {
            t.Errorf(Sprintf!("ChecksumIEEE(%q) = %x, want %x", g.inp, got as i64, g.ieee as i64));
        }
    }
}}

test!{ fn TestStreaming(t) {
    // Accumulate in pieces, should match one-shot.
    let __golden = golden();
    for (_, g) in range!(__golden) {
        let mut h = hash::crc32::NewIEEE();
        let half = g.inp.len() / 2;
        h.Write(&g.inp.as_bytes()[..half]);
        h.Write(&g.inp.as_bytes()[half..]);
        let got = h.Sum32();
        if got != g.ieee {
            t.Errorf(Sprintf!("streaming(%q) = %x, want %x", g.inp, got as i64, g.ieee as i64));
        }
    }
}}

test!{ fn TestReset(t) {
    let mut h = hash::crc32::NewIEEE();
    h.Write(b"one");
    h.Reset();
    h.Write(b"abc");
    let got = h.Sum32();
    if got != 0x352441c2 {
        t.Errorf(Sprintf!("after Reset: %x, want 0x352441c2", got as i64));
    }
}}
