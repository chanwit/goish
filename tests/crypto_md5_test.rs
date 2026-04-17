// Port of go1.25.5 src/crypto/md5/md5_test.go TestGolden.
//
// Skipped: TestGoldenMarshal (BinaryMarshaler is not in goish's surface),
// TestLarge (1 GiB stream; takes too long for CI), cryptotest-driven
// tests (internal Go testing infrastructure).
//
// Covered: RFC 1321 + Go's extended golden vectors. For each vector we
// test Sum (one-shot), streaming Write + Sum, split-mid-message Write,
// and Reset between runs.

#![allow(non_snake_case)]
use goish::prelude::*;

struct GoldenMd5 {
    out: &'static str,
    inp: &'static str,
}

fn golden() -> Vec<GoldenMd5> {
    vec![
        GoldenMd5 { out: "d41d8cd98f00b204e9800998ecf8427e", inp: "" },
        GoldenMd5 { out: "0cc175b9c0f1b6a831c399e269772661", inp: "a" },
        GoldenMd5 { out: "187ef4436122d1cc2f40dc2b92f0eba0", inp: "ab" },
        GoldenMd5 { out: "900150983cd24fb0d6963f7d28e17f72", inp: "abc" },
        GoldenMd5 { out: "e2fc714c4727ee9395f324cd2e7f331f", inp: "abcd" },
        GoldenMd5 { out: "ab56b4d92b40713acc5af89985d4b786", inp: "abcde" },
        GoldenMd5 { out: "e80b5017098950fc58aad83c8c14978e", inp: "abcdef" },
        GoldenMd5 { out: "7ac66c0f148de9519b8bd264312c4d64", inp: "abcdefg" },
        GoldenMd5 { out: "e8dc4081b13434b45189a720b77b6818", inp: "abcdefgh" },
        GoldenMd5 { out: "8aa99b1f439ff71293e95357bac6fd94", inp: "abcdefghi" },
        GoldenMd5 { out: "a925576942e94b2ef57a066101b48876", inp: "abcdefghij" },
        GoldenMd5 { out: "d747fc1719c7eacb84058196cfe56d57", inp: "Discard medicine more than two years old." },
        GoldenMd5 { out: "bff2dcb37ef3a44ba43ab144768ca837", inp: "He who has a shady past knows that nice guys finish last." },
        GoldenMd5 { out: "0441015ecb54a7342d017ed1bcfdbea5", inp: "I wouldn't marry him with a ten foot pole." },
        GoldenMd5 { out: "9e3cac8e9e9757a60c3ea391130d3689", inp: "Free! Free!/A trip/to Mars/for 900/empty jars/Burma Shave" },
        GoldenMd5 { out: "a0f04459b031f916a59a35cc482dc039", inp: "The days of the digital watch are numbered.  -Tom Stoppard" },
        GoldenMd5 { out: "e7a48e0fe884faf31475d2a04b1362cc", inp: "Nepal premier won't resign." },
        GoldenMd5 { out: "637d2fe925c07c113800509964fb0e06", inp: "For every action there is an equal and opposite government program." },
        GoldenMd5 { out: "834a8d18d5c6562119cf4c7f5086cb71", inp: "His money is twice tainted: 'taint yours and 'taint mine." },
        GoldenMd5 { out: "de3a4d2fd6c73ec2db2abad23b444281", inp: "There is no reason for any individual to have a computer in their home. -Ken Olsen, 1977" },
        GoldenMd5 { out: "acf203f997e2cf74ea3aff86985aefaf", inp: "It's a tiny change to the code and not completely disgusting. - Bob Manchek" },
        GoldenMd5 { out: "e1c1384cb4d2221dfdd7c795a4222c9a", inp: "size:  a.out:  bad magic" },
        GoldenMd5 { out: "c90f3ddecc54f34228c063d7525bf644", inp: "The major problem is with sendmail.  -Mark Horton" },
        GoldenMd5 { out: "cdf7ab6c1fd49bd9933c43f3ea5af185", inp: "Give me a rock, paper and scissors and I will move the world.  CCFestoon" },
        GoldenMd5 { out: "83bc85234942fc883c063cbd7f0ad5d0", inp: "If the enemy is within range, then so are you." },
        GoldenMd5 { out: "277cbe255686b48dd7e8f389394d9299", inp: "It's well we cannot hear the screams/That we create in others' dreams." },
        GoldenMd5 { out: "fd3fb0a7ffb8af16603f3d3af98f8e1f", inp: "You remind me of a TV show, but that's all right: I watch it anyway." },
        GoldenMd5 { out: "469b13a78ebf297ecda64d4723655154", inp: "C is as portable as Stonehedge!!" },
        GoldenMd5 { out: "63eb3a2f466410104731c4b037600110", inp: "Even if I could be Shakespeare, I think I should still choose to be Faraday. - A. Huxley" },
        GoldenMd5 { out: "72c2ed7592debca1c90fc0100f931a2f", inp: "The fugacity of a constituent in a mixture of gases at a given temperature is proportional to its mole fraction.  Lewis-Randall Rule" },
        GoldenMd5 { out: "132f7619d33b523b1d9e5bd8e0928355", inp: "How can you write a big system without C++?  -Paul Glick" },
    ]
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

test!{ fn TestGolden(t) {
    let __golden = golden();
    for (_, g) in range!(__golden) {
        let sum = crypto::md5::Sum(g.inp.as_bytes());
        let hex = to_hex(&sum);
        if hex != g.out {
            t.Errorf(Sprintf!("Sum function: md5(%s) = %s want %s", g.inp, hex, g.out));
            continue;
        }
        // Streaming: write input, Sum, compare.
        let mut c = crypto::md5::New();
        c.Write(g.inp.as_bytes());
        let s1 = to_hex(&c.Sum(&[]));
        if s1 != g.out {
            t.Errorf(Sprintf!("streaming: md5(%s) = %s want %s", g.inp, s1, g.out));
        }
        // Split write: first half, Sum intermediate (discard), second half.
        c.Reset();
        let half = g.inp.len() / 2;
        c.Write(&g.inp.as_bytes()[..half]);
        let _intermediate = c.Sum(&[]);  // Go tests this mid-Sum doesn't corrupt state
        c.Write(&g.inp.as_bytes()[half..]);
        let s2 = to_hex(&c.Sum(&[]));
        if s2 != g.out {
            t.Errorf(Sprintf!("split write: md5(%s) = %s want %s", g.inp, s2, g.out));
        }
        // Reset + rewrite should give same result.
        c.Reset();
        c.Write(g.inp.as_bytes());
        let s3 = to_hex(&c.Sum(&[]));
        if s3 != g.out {
            t.Errorf(Sprintf!("post-reset: md5(%s) = %s want %s", g.inp, s3, g.out));
        }
    }
}}

test!{ fn TestSize(t) {
    if crypto::md5::Size != 16 {
        t.Errorf(Sprintf!("Size = %d, want 16", crypto::md5::Size));
    }
}}

test!{ fn TestBlockSize(t) {
    if crypto::md5::BlockSize != 64 {
        t.Errorf(Sprintf!("BlockSize = %d, want 64", crypto::md5::BlockSize));
    }
}}
