// Port of go1.25.5 src/net/mail/message_test.go — addr-spec, display-
// name forms, comma-separated lists. RFC 2047 encoded-word + group
// addresses are deferred (goish implements the common subset).

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::net::mail;

// ── TestAddressParsing (core subset) ────────────────────────────────

test!{ fn TestAddressParsing(t) {
    struct Case {
        input: &'static str,
        name: &'static str,
        address: &'static str,
    }
    let cases = [
        Case { input: "jdoe@machine.example",                       name: "",          address: "jdoe@machine.example" },
        Case { input: "John Doe <jdoe@machine.example>",            name: "John Doe",  address: "jdoe@machine.example" },
        Case { input: "\"Joe Q. Public\" <john.q.public@example.com>",
               name: "Joe Q. Public", address: "john.q.public@example.com" },
        // Comment in display name (stripped).
        Case { input: "John (middle) Doe <jdoe@machine.example>",
               name: "John Doe", address: "jdoe@machine.example" },
        // Quoted-string display name containing what looks like a comment.
        Case { input: "\"John (middle) Doe\" <jdoe@machine.example>",
               name: "John (middle) Doe", address: "jdoe@machine.example" },
        Case { input: "\"John <middle> Doe\" <jdoe@machine.example>",
               name: "John <middle> Doe", address: "jdoe@machine.example" },
        // RFC 5322 A.6.1 — phrase with dot.
        Case { input: "Joe Q. Public <john.q.public@example.com>",
               name: "Joe Q. Public", address: "john.q.public@example.com" },
    ];
    for c in &cases {
        let (a, err) = mail::ParseAddress(c.input);
        if err != nil {
            t.Errorf(Sprintf!("ParseAddress(%s): error: %s", c.input, err));
            continue;
        }
        if a.Name != c.name {
            t.Errorf(Sprintf!("ParseAddress(%s).Name = %s, want %s", c.input, a.Name, c.name));
        }
        if a.Address != c.address {
            t.Errorf(Sprintf!("ParseAddress(%s).Address = %s, want %s", c.input, a.Address, c.address));
        }
    }
}}

// ── TestAddressParsingList (comma-separated) ────────────────────────

test!{ fn TestAddressParsingList(t) {
    let input = "Mary Smith <mary@x.test>, jdoe@example.org, Who? <one@y.test>";
    let (list, err) = mail::ParseAddressList(input);
    if err != nil { t.Fatal(&Sprintf!("ParseAddressList: %s", err)); }
    if list.len() != 3 {
        t.Fatal(&Sprintf!("list len = %d, want 3", list.len() as i64));
    }
    let want: [(&str, &str); 3] = [
        ("Mary Smith", "mary@x.test"),
        ("",           "jdoe@example.org"),
        ("Who?",       "one@y.test"),
    ];
    for i in 0..3 {
        if list[i].Name != want[i].0 {
            t.Errorf(Sprintf!("[%d].Name = %s, want %s", i as i64, list[i].Name, want[i].0));
        }
        if list[i].Address != want[i].1 {
            t.Errorf(Sprintf!("[%d].Address = %s, want %s", i as i64, list[i].Address, want[i].1));
        }
    }
}}

// ── TestAddressParsingError (invalid inputs reject) ────────────────

test!{ fn TestAddressParsingError(t) {
    let bad = [
        "",
        "not-an-email",
        "foo@",
        "@bar.com",
        "John Doe <>",
        "John Doe <not-an-email>",
    ];
    for s in &bad {
        let (_a, err) = mail::ParseAddress(s);
        if err == nil {
            t.Errorf(Sprintf!("ParseAddress(%s) = no error, want error", s));
        }
    }
}}

// ── TestAddressParsingDomainLiteral (RFC 5322 domain-literal) ───────

test!{ fn TestAddressParsingDomainLiteral(t) {
    // "foo@[127.0.0.1]" — domain is a literal in square brackets.
    let (a, err) = mail::ParseAddress("foo@[127.0.0.1]");
    if err != nil { t.Fatal(&Sprintf!("ParseAddress: %s", err)); }
    if a.Address != "foo@[127.0.0.1]" {
        t.Errorf(Sprintf!("Address = %s, want foo@[127.0.0.1]", a.Address));
    }
}}

// ── TestAddressParsingQuotedLocal (quoted-string local part) ────────

test!{ fn TestAddressParsingQuotedLocal(t) {
    let (a, err) = mail::ParseAddress("\"very.unusual\"@example.com");
    if err != nil { t.Fatal(&Sprintf!("ParseAddress: %s", err)); }
    if a.Address != "very.unusual@example.com" {
        t.Errorf(Sprintf!("Address = %s", a.Address));
    }
}}

// ── TestAddressParsingAngleOnlyForm ────────────────────────────────

test!{ fn TestAddressParsingAngleOnlyForm(t) {
    let (a, err) = mail::ParseAddress("<boss@nil.test>");
    if err != nil { t.Fatal(&Sprintf!("ParseAddress: %s", err)); }
    if a.Address != "boss@nil.test" {
        t.Errorf(Sprintf!("Address = %s", a.Address));
    }
    if !a.Name.is_empty() {
        t.Errorf(Sprintf!("Name = %s, want empty", a.Name));
    }
}}

// ── TestAddressString (format round-trip) ───────────────────────────

test!{ fn TestAddressString(t) {
    struct Case {
        name: &'static str,
        address: &'static str,
        want: &'static str,
    }
    let cases = [
        Case { name: "",            address: "bob@example.com", want: "bob@example.com" },
        Case { name: "Bob",         address: "bob@example.com", want: "Bob <bob@example.com>" },
        Case { name: "Bob Smith",   address: "bob@example.com", want: "Bob Smith <bob@example.com>" },
        Case { name: "Joe; Public", address: "joe@example.com", want: "\"Joe; Public\" <joe@example.com>" },
    ];
    for c in &cases {
        let a = MailAddress!{Name: c.name, Address: c.address};
        let got = a.String();
        if got != c.want {
            t.Errorf(Sprintf!("String(%s,%s) = %s, want %s", c.name, c.address, got, c.want));
        }
    }
}}
