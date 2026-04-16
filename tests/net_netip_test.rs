// Port of go1.25.5 src/net/netip/netip_test.go — covers the goish
// Addr/AddrPort/Prefix surface. Test input tables are copied verbatim
// from the Go test file; helper constructors (MkAddr/Mk128/Z4/Z6noz/
// unique.Make) are inlined into direct AddrFrom4/AddrFrom16/WithZone
// calls to keep the porting 1:1 at the data level.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::net::netip;

// ── TestParseAddr: every valid input round-trips ────────────────────

test!{ fn TestParseAddr_ValidInputs(t) {
    struct Case {
        input: &'static str,
        want: &'static str, // canonical String() output
    }
    let cases = [
        Case { input: "0.0.0.0",                                              want: "0.0.0.0" },
        Case { input: "192.168.140.255",                                      want: "192.168.140.255" },
        Case { input: "::",                                                    want: "::" },
        Case { input: "::1",                                                   want: "::1" },
        Case { input: "fd7a:115c:a1e0:ab12:4843:cd96:626b:430b",               want: "fd7a:115c:a1e0:ab12:4843:cd96:626b:430b" },
        Case { input: "fd7a:115c::626b:430b",                                  want: "fd7a:115c::626b:430b" },
        Case { input: "fd7a:115c:a1e0:ab12:4843:cd96::",                      want: "fd7a:115c:a1e0:ab12:4843:cd96::" },
        Case { input: "fd7a:115c:a1e0:ab12:4843:cd96:626b::",                 want: "fd7a:115c:a1e0:ab12:4843:cd96:626b:0" },
        Case { input: "fd7a:115c:a1e0::4843:cd96:626b:430b",                   want: "fd7a:115c:a1e0:0:4843:cd96:626b:430b" },
        Case { input: "FD9E:1A04:F01D::1",                                     want: "fd9e:1a04:f01d::1" },
    ];
    for c in &cases {
        let (got, err) = netip::ParseAddr(c.input);
        if err != nil {
            t.Errorf(Sprintf!("ParseAddr(%s): unexpected error: %s", c.input, err));
            continue;
        }
        let s = got.String();
        if s != c.want {
            t.Errorf(Sprintf!("ParseAddr(%s).String() = %s, want %s", c.input, s, c.want));
        }
        // Round-trip: Parse(ip.String()) must equal original.
        let (got2, err2) = netip::ParseAddr(&s);
        if err2 != nil {
            t.Errorf(Sprintf!("round-trip ParseAddr(%s): error: %s", s, err2));
            continue;
        }
        if got2.String() != s {
            t.Errorf(Sprintf!("round-trip mismatch for %s: got %s", c.input, got2.String()));
        }
    }
}}

// ── TestParseAddr: every invalid input errors ───────────────────────

test!{ fn TestParseAddr_InvalidInputs(t) {
    let invalid = [
        "",
        "bad",
        "1234",
        "1.2.3.4%eth0",
        ".1.2.3",
        "1.2.3.",
        "1..2.3",
        "1.2.3.4.5",
        "192.168.300.1",
        "192.168.0.1.5.6",
        "1:2:3:4:5:6:7",
        "1:2:3:4:5:6:7:8:9",
        "fe801::1",
        "fe80:tail:scal:e::",
        "::ffff:192.168.140.bad",
        "fe80::1::1",
        "fe80:1?:1",
        "fe80:",
    ];
    for s in &invalid {
        let (_a, err) = netip::ParseAddr(s);
        if err == nil {
            t.Errorf(Sprintf!("ParseAddr(%s) = no error, want error", s));
        }
    }
}}

// ── TestAddrFromSlice: 4-byte → IPv4, 16-byte → IPv6, else fail ─────

test!{ fn TestAddrFromSlice(t) {
    let (a, ok) = netip::AddrFromSlice(&[10, 0, 0, 1]);
    if !ok { t.Fatal("AddrFromSlice 4-byte: ok=false"); }
    if a.String() != "10.0.0.1" { t.Errorf(Sprintf!("4-byte String = %s", a.String())); }
    if !a.Is4() { t.Errorf(Sprintf!("4-byte Is4() false")); }

    let mut v6 = [0u8; 16];
    v6[0] = 0xfe; v6[1] = 0x80; v6[15] = 0x01;
    let (b, ok) = netip::AddrFromSlice(&v6);
    if !ok { t.Fatal("AddrFromSlice 16-byte: ok=false"); }
    if b.String() != "fe80::1" { t.Errorf(Sprintf!("16-byte String = %s", b.String())); }
    if !b.Is6() { t.Errorf(Sprintf!("16-byte Is6() false")); }

    let (_, ok) = netip::AddrFromSlice(&[0, 1, 2]);
    if ok { t.Errorf(Sprintf!("3-byte AddrFromSlice ok = true, want false")); }
}}

// ── TestIs4AndIs6 ────────────────────────────────────────────────────

test!{ fn TestIs4AndIs6(t) {
    let a = netip::MustParseAddr("10.0.0.1");
    if !a.Is4() { t.Errorf(Sprintf!("Is4() false for IPv4")); }
    if a.Is6()  { t.Errorf(Sprintf!("Is6() true for IPv4")); }

    let b = netip::MustParseAddr("::1");
    if b.Is4() { t.Errorf(Sprintf!("Is4() true for IPv6 loopback")); }
    if !b.Is6() { t.Errorf(Sprintf!("Is6() false for IPv6 loopback")); }
}}

// ── TestIs4In6 ───────────────────────────────────────────────────────

test!{ fn TestIs4In6(t) {
    let a = netip::MustParseAddr("::ffff:192.168.140.255");
    if !a.Is4In6() { t.Errorf(Sprintf!("Is4In6() false for 4-in-6")); }
    if a.Is4() { t.Errorf(Sprintf!("Is4() true for 4-in-6 (should be v6)")); }

    let unmapped = a.Unmap();
    if !unmapped.Is4() { t.Errorf(Sprintf!("Unmap did not produce v4")); }
    if unmapped.String() != "192.168.140.255" {
        t.Errorf(Sprintf!("Unmap.String() = %s, want 192.168.140.255", unmapped.String()));
    }
}}

// ── TestIPProperties: Unspecified / Loopback / Multicast / LinkLocal / Private ──

test!{ fn TestIPProperties(t) {
    let unspec4 = netip::MustParseAddr("0.0.0.0");
    if !unspec4.IsUnspecified() { t.Errorf(Sprintf!("0.0.0.0 IsUnspecified false")); }
    let unspec6 = netip::MustParseAddr("::");
    if !unspec6.IsUnspecified() { t.Errorf(Sprintf!(":: IsUnspecified false")); }

    let lo4 = netip::MustParseAddr("127.0.0.1");
    if !lo4.IsLoopback() { t.Errorf(Sprintf!("127.0.0.1 IsLoopback false")); }
    let lo6 = netip::MustParseAddr("::1");
    if !lo6.IsLoopback() { t.Errorf(Sprintf!("::1 IsLoopback false")); }

    let mc4 = netip::MustParseAddr("239.0.0.1");
    if !mc4.IsMulticast() { t.Errorf(Sprintf!("239.0.0.1 IsMulticast false")); }
    let mc6 = netip::MustParseAddr("ff02::1");
    if !mc6.IsMulticast() { t.Errorf(Sprintf!("ff02::1 IsMulticast false")); }

    let ll4 = netip::MustParseAddr("169.254.1.1");
    if !ll4.IsLinkLocalUnicast() { t.Errorf(Sprintf!("169.254.1.1 IsLinkLocalUnicast false")); }
    let ll6 = netip::MustParseAddr("fe80::1");
    if !ll6.IsLinkLocalUnicast() { t.Errorf(Sprintf!("fe80::1 IsLinkLocalUnicast false")); }

    let priv4 = netip::MustParseAddr("10.0.0.1");
    if !priv4.IsPrivate() { t.Errorf(Sprintf!("10.0.0.1 IsPrivate false")); }
    let pub4 = netip::MustParseAddr("8.8.8.8");
    if pub4.IsPrivate() { t.Errorf(Sprintf!("8.8.8.8 IsPrivate true")); }
}}

// ── TestAddrCompare (subset) ─────────────────────────────────────────

test!{ fn TestAddrLessCompare(t) {
    let a = netip::MustParseAddr("1.2.3.4");
    let b = netip::MustParseAddr("1.2.3.5");
    let c = netip::MustParseAddr("::1");
    if !a.Less(&b) { t.Errorf(Sprintf!("1.2.3.4 < 1.2.3.5 false")); }
    if !a.Less(&c) { t.Errorf(Sprintf!("v4 < v6 false")); }
    if a.Compare(&a) != 0 { t.Errorf(Sprintf!("self Compare != 0")); }
}}

// ── TestParseAddrPort ────────────────────────────────────────────────

test!{ fn TestParseAddrPort(t) {
    struct Case { input: &'static str, want: &'static str }
    let cases = [
        Case { input: "1.2.3.4:80",      want: "1.2.3.4:80" },
        Case { input: "[::1]:80",         want: "[::1]:80" },
        Case { input: "[fe80::1]:443",    want: "[fe80::1]:443" },
    ];
    for c in &cases {
        let (ap, err) = netip::ParseAddrPort(c.input);
        if err != nil {
            t.Errorf(Sprintf!("ParseAddrPort(%s): unexpected error: %s", c.input, err));
            continue;
        }
        if ap.String() != c.want {
            t.Errorf(Sprintf!("ParseAddrPort(%s).String() = %s, want %s", c.input, ap.String(), c.want));
        }
    }
    let bad = ["1.2.3.4", "::1:80", "[::1]", "1.2.3.4:abc"];
    for b in &bad {
        let (_ap, err) = netip::ParseAddrPort(b);
        if err == nil {
            t.Errorf(Sprintf!("ParseAddrPort(%s) = no error, want error", b));
        }
    }
}}

// ── TestParsePrefix / TestPrefixString ──────────────────────────────

test!{ fn TestParsePrefix(t) {
    struct Case { input: &'static str, want: &'static str }
    let cases = [
        Case { input: "192.168.0.0/16",    want: "192.168.0.0/16" },
        Case { input: "10.0.0.0/8",        want: "10.0.0.0/8" },
        Case { input: "::1/128",            want: "::1/128" },
        Case { input: "2001:db8::/32",      want: "2001:db8::/32" },
        Case { input: "fe80::/10",          want: "fe80::/10" },
    ];
    for c in &cases {
        let (p, err) = netip::ParsePrefix(c.input);
        if err != nil {
            t.Errorf(Sprintf!("ParsePrefix(%s): error: %s", c.input, err));
            continue;
        }
        if p.String() != c.want {
            t.Errorf(Sprintf!("ParsePrefix(%s).String() = %s, want %s", c.input, p.String(), c.want));
        }
    }
    // Invalid
    let bad = ["1.2.3.4",  "1.2.3.4/33", "::1/129", "1.2.3.4/-1", "1.2.3.4/a"];
    for b in &bad {
        let (_p, err) = netip::ParsePrefix(b);
        if err == nil {
            t.Errorf(Sprintf!("ParsePrefix(%s) = no error, want error", b));
        }
    }
}}

// ── TestPrefixMasked: mask off host bits ────────────────────────────

test!{ fn TestPrefixMasked(t) {
    struct Case { input: &'static str, want: &'static str }
    let cases = [
        Case { input: "10.1.2.3/8",       want: "10.0.0.0/8" },
        Case { input: "192.168.1.100/24", want: "192.168.1.0/24" },
        Case { input: "2001:db8:a::1/32", want: "2001:db8::/32" },
    ];
    for c in &cases {
        let p = netip::MustParsePrefix(c.input);
        let m = p.Masked();
        if m.String() != c.want {
            t.Errorf(Sprintf!("%s.Masked() = %s, want %s", c.input, m.String(), c.want));
        }
    }
}}

// ── TestPrefixContains ──────────────────────────────────────────────

test!{ fn TestPrefixContains(t) {
    let p = netip::MustParsePrefix("10.0.0.0/8");
    if !p.Contains(netip::MustParseAddr("10.1.2.3")) {
        t.Errorf(Sprintf!("10.0.0.0/8.Contains(10.1.2.3) = false"));
    }
    if p.Contains(netip::MustParseAddr("11.0.0.0")) {
        t.Errorf(Sprintf!("10.0.0.0/8.Contains(11.0.0.0) = true"));
    }
    let p6 = netip::MustParsePrefix("2001:db8::/32");
    if !p6.Contains(netip::MustParseAddr("2001:db8::1")) {
        t.Errorf(Sprintf!("2001:db8::/32.Contains(2001:db8::1) = false"));
    }
}}

// ── TestAddrNextPrev ────────────────────────────────────────────────

test!{ fn TestAddrNextPrev(t) {
    let a = netip::MustParseAddr("1.2.3.4");
    if a.Next().String() != "1.2.3.5" {
        t.Errorf(Sprintf!("1.2.3.4.Next() = %s", a.Next().String()));
    }
    if a.Prev().String() != "1.2.3.3" {
        t.Errorf(Sprintf!("1.2.3.4.Prev() = %s", a.Prev().String()));
    }
    let last = netip::MustParseAddr("255.255.255.255");
    if last.Next().IsValid() {
        t.Errorf(Sprintf!("255.255.255.255.Next() should be invalid"));
    }
}}

// ── TestAddrFrom4 / TestAddrFrom16 ──────────────────────────────────

test!{ fn TestAddrFrom4(t) {
    let a = netip::AddrFrom4([1, 2, 3, 4]);
    if a.String() != "1.2.3.4" { t.Errorf(Sprintf!("AddrFrom4 = %s", a.String())); }
    if !a.Is4() { t.Errorf(Sprintf!("AddrFrom4 not v4")); }
    let bytes = a.As4();
    if bytes != [1u8, 2, 3, 4] { t.Errorf(Sprintf!("As4 round-trip broken")); }
}}

test!{ fn TestAddrFrom16(t) {
    let mut b = [0u8; 16];
    b[0] = 0x20; b[1] = 0x01; b[2] = 0x0d; b[3] = 0xb8; b[15] = 1;
    let a = netip::AddrFrom16(b);
    if a.String() != "2001:db8::1" { t.Errorf(Sprintf!("AddrFrom16 = %s", a.String())); }
    if !a.Is6() { t.Errorf(Sprintf!("AddrFrom16 not v6")); }
    let out = a.As16();
    for i in 0..16 {
        if out[i] != b[i] {
            t.Errorf(Sprintf!("As16 round-trip byte %d: got %d want %d", i as i64, out[i] as i64, b[i] as i64));
            break;
        }
    }
}}

// ── TestAddrZone ────────────────────────────────────────────────────

test!{ fn TestAddrZone(t) {
    let a = netip::MustParseAddr("fe80::1%eth0");
    if a.Zone() != "eth0" { t.Errorf(Sprintf!("Zone = %s, want eth0", a.Zone())); }
    if a.String() != "fe80::1%eth0" {
        t.Errorf(Sprintf!("String = %s, want fe80::1%%eth0", a.String()));
    }
    let b = a.WithZone("eth1");
    if b.Zone() != "eth1" { t.Errorf(Sprintf!("WithZone: %s, want eth1", b.Zone())); }
    // v4 drops zone.
    let c = netip::MustParseAddr("1.2.3.4").WithZone("zzz");
    if c.Zone() != "" { t.Errorf(Sprintf!("v4 WithZone kept zone %s", c.Zone())); }
}}
