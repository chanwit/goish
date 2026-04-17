// Port of go1.25.5 src/crypto/sha1/sha1_test.go TestGolden.
//
// Skipped: TestGoldenMarshal, TestBlockGeneric, ConstantTimeSum,
// cryptotest driver.

#![allow(non_snake_case)]
use goish::prelude::*;

struct GoldenSha1 {
    out: &'static str,
    inp: &'static str,
}

fn golden() -> Vec<GoldenSha1> {
    vec![
        GoldenSha1 { out: "76245dbf96f661bd221046197ab8b9f063f11bad",
            inp: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n" },
        GoldenSha1 { out: "da39a3ee5e6b4b0d3255bfef95601890afd80709", inp: "" },
        GoldenSha1 { out: "86f7e437faa5a7fce15d1ddcb9eaeaea377667b8", inp: "a" },
        GoldenSha1 { out: "da23614e02469a0d7c7bd1bdab5c9c474b1904dc", inp: "ab" },
        GoldenSha1 { out: "a9993e364706816aba3e25717850c26c9cd0d89d", inp: "abc" },
        GoldenSha1 { out: "81fe8bfe87576c3ecb22426f8e57847382917acf", inp: "abcd" },
        GoldenSha1 { out: "03de6c570bfe24bfc328ccd7ca46b76eadaf4334", inp: "abcde" },
        GoldenSha1 { out: "1f8ac10f23c5b5bc1167bda84b833e5c057a77d2", inp: "abcdef" },
        GoldenSha1 { out: "2fb5e13419fc89246865e7a324f476ec624e8740", inp: "abcdefg" },
        GoldenSha1 { out: "425af12a0743502b322e93a015bcf868e324d56a", inp: "abcdefgh" },
        GoldenSha1 { out: "c63b19f1e4c8b5f76b25c49b8b87f57d8e4872a1", inp: "abcdefghi" },
        GoldenSha1 { out: "d68c19a0a345b7eab78d5e11e991c026ec60db63", inp: "abcdefghij" },
        GoldenSha1 { out: "ebf81ddcbe5bf13aaabdc4d65354fdf2044f38a7", inp: "Discard medicine more than two years old." },
        GoldenSha1 { out: "e5dea09392dd886ca63531aaa00571dc07554bb6", inp: "He who has a shady past knows that nice guys finish last." },
        GoldenSha1 { out: "45988f7234467b94e3e9494434c96ee3609d8f8f", inp: "I wouldn't marry him with a ten foot pole." },
        GoldenSha1 { out: "55dee037eb7460d5a692d1ce11330b260e40c988", inp: "Free! Free!/A trip/to Mars/for 900/empty jars/Burma Shave" },
        GoldenSha1 { out: "b7bc5fb91080c7de6b582ea281f8a396d7c0aee8", inp: "The days of the digital watch are numbered.  -Tom Stoppard" },
        GoldenSha1 { out: "c3aed9358f7c77f523afe86135f06b95b3999797", inp: "Nepal premier won't resign." },
        GoldenSha1 { out: "6e29d302bf6e3a5e4305ff318d983197d6906bb9", inp: "For every action there is an equal and opposite government program." },
        GoldenSha1 { out: "597f6a540010f94c15d71806a99a2c8710e747bd", inp: "His money is twice tainted: 'taint yours and 'taint mine." },
        GoldenSha1 { out: "6859733b2590a8a091cecf50086febc5ceef1e80", inp: "There is no reason for any individual to have a computer in their home. -Ken Olsen, 1977" },
        GoldenSha1 { out: "514b2630ec089b8aee18795fc0cf1f4860cdacad", inp: "It's a tiny change to the code and not completely disgusting. - Bob Manchek" },
        GoldenSha1 { out: "c5ca0d4a7b6676fc7aa72caa41cc3d5df567ed69", inp: "size:  a.out:  bad magic" },
        GoldenSha1 { out: "74c51fa9a04eadc8c1bbeaa7fc442f834b90a00a", inp: "The major problem is with sendmail.  -Mark Horton" },
        GoldenSha1 { out: "0b4c4ce5f52c3ad2821852a8dc00217fa18b8b66", inp: "Give me a rock, paper and scissors and I will move the world.  CCFestoon" },
        GoldenSha1 { out: "3ae7937dd790315beb0f48330e8642237c61550a", inp: "If the enemy is within range, then so are you." },
        GoldenSha1 { out: "410a2b296df92b9a47412b13281df8f830a9f44b", inp: "It's well we cannot hear the screams/That we create in others' dreams." },
        GoldenSha1 { out: "841e7c85ca1adcddbdd0187f1289acb5c642f7f5", inp: "You remind me of a TV show, but that's all right: I watch it anyway." },
        GoldenSha1 { out: "163173b825d03b952601376b25212df66763e1db", inp: "C is as portable as Stonehedge!!" },
        GoldenSha1 { out: "32b0377f2687eb88e22106f133c586ab314d5279", inp: "Even if I could be Shakespeare, I think I should still choose to be Faraday. - A. Huxley" },
        GoldenSha1 { out: "0885aaf99b569542fd165fa44e322718f4a984e0", inp: "The fugacity of a constituent in a mixture of gases at a given temperature is proportional to its mole fraction.  Lewis-Randall Rule" },
        GoldenSha1 { out: "6627d6904d71420b0bf3886ab629623538689f45", inp: "How can you write a big system without C++?  -Paul Glick" },
    ]
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes { s.push_str(&format!("{:02x}", b)); }
    s
}

test!{ fn TestGolden(t) {
    let __golden = golden();
    for (_, g) in range!(__golden) {
        let sum = crypto::sha1::Sum(g.inp.as_bytes());
        let hex = to_hex(&sum);
        if hex != g.out {
            t.Errorf(Sprintf!("Sum: sha1(%s) = %s want %s", g.inp, hex, g.out));
            continue;
        }
        let mut c = crypto::sha1::New();
        c.Write(g.inp.as_bytes());
        let s1 = to_hex(&c.Sum(&[]));
        if s1 != g.out {
            t.Errorf(Sprintf!("streaming: sha1(%s) = %s want %s", g.inp, s1, g.out));
        }
        c.Reset();
        let half = g.inp.len() / 2;
        c.Write(&g.inp.as_bytes()[..half]);
        let _ = c.Sum(&[]);
        c.Write(&g.inp.as_bytes()[half..]);
        let s2 = to_hex(&c.Sum(&[]));
        if s2 != g.out {
            t.Errorf(Sprintf!("split write: sha1(%s) = %s want %s", g.inp, s2, g.out));
        }
    }
}}

test!{ fn TestSize(t) {
    if crypto::sha1::Size != 20 {
        t.Errorf(Sprintf!("Size = %d, want 20", crypto::sha1::Size));
    }
}}

test!{ fn TestBlockSize(t) {
    if crypto::sha1::BlockSize != 64 {
        t.Errorf(Sprintf!("BlockSize = %d, want 64", crypto::sha1::BlockSize));
    }
}}
