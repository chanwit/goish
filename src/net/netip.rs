// netip: Go's net/netip — value-typed IP addresses.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   addr, err := netip.ParseAddr(s)     let (addr, err) = netip::ParseAddr(s);
//   addr.Is4()                          addr.Is4()
//   addr.Is6()                          addr.Is6()
//   addr.String()                       addr.String()
//   netip.MustParseAddr(s)              netip::MustParseAddr(s)
//   netip.AddrFrom4([4]byte{..})        netip::AddrFrom4([u8; 4])
//   ap, err := netip.ParseAddrPort(s)   let (ap, err) = netip::ParseAddrPort(s)
//   p, err  := netip.ParsePrefix(s)     let (p, err)  = netip::ParsePrefix(s)
//
// Same-shape API as Go. All three types are value-semantic (Copy) — match
// Go's zero-allocation netip types.

use crate::errors::{error, nil, New};
use crate::types::string;

// ── Addr ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Addr {
    // 128 bits; 4in6-mapped form when holding IPv4.
    hi: u64,
    lo: u64,
    // true iff the address was constructed as IPv4 (influences String()).
    is4: bool,
    // RFC 4007 zone. Only meaningful when !is4 and non-empty.
    zone: string,
    // invalid (zero) if false.
    valid: bool,
}

impl Addr {
    pub fn IsValid(&self) -> bool { self.valid }
    pub fn Is4(&self) -> bool { self.valid && self.is4 }
    pub fn Is6(&self) -> bool { self.valid && !self.is4 }

    pub fn Is4In6(&self) -> bool {
        self.valid && !self.is4 && self.hi == 0 && (self.lo >> 32) == 0xffff
    }

    pub fn Unmap(&self) -> Addr {
        if self.Is4In6() {
            let b = (self.lo & 0xffff_ffff) as u32;
            return AddrFrom4([(b >> 24) as u8, (b >> 16) as u8, (b >> 8) as u8, b as u8]);
        }
        self.clone()
    }

    pub fn As4(&self) -> [u8; 4] {
        if self.is4 || self.Is4In6() {
            let b = (self.lo & 0xffff_ffff) as u32;
            return [(b >> 24) as u8, (b >> 16) as u8, (b >> 8) as u8, b as u8];
        }
        panic!("netip: As4 called on IPv6 address");
    }

    pub fn As16(&self) -> [u8; 16] {
        if self.is4 {
            // 4in6-mapped: ::ffff:a.b.c.d
            let b = (self.lo & 0xffff_ffff) as u32;
            let mut out = [0u8; 16];
            out[10] = 0xff;
            out[11] = 0xff;
            out[12] = (b >> 24) as u8;
            out[13] = (b >> 16) as u8;
            out[14] = (b >> 8) as u8;
            out[15] = b as u8;
            return out;
        }
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&self.hi.to_be_bytes());
        out[8..].copy_from_slice(&self.lo.to_be_bytes());
        out
    }

    pub fn AsSlice(&self) -> Vec<u8> {
        if !self.valid { return Vec::new(); }
        if self.is4 { return self.As4().to_vec(); }
        self.As16().to_vec()
    }

    pub fn BitLen(&self) -> i64 {
        if !self.valid { return 0; }
        if self.is4 { 32 } else { 128 }
    }

    pub fn Zone(&self) -> string { self.zone.clone() }

    pub fn WithZone(&self, zone: impl Into<string>) -> Addr {
        if self.is4 { return self.clone(); } // Go drops zones on IPv4.
        let mut a = self.clone();
        a.zone = zone.into();
        a
    }

    pub fn IsUnspecified(&self) -> bool {
        self.valid && self.hi == 0 && self.lo == 0
    }

    pub fn IsLoopback(&self) -> bool {
        if !self.valid { return false; }
        if self.is4 {
            let b = self.As4();
            return b[0] == 127;
        }
        self.hi == 0 && self.lo == 1
    }

    pub fn IsMulticast(&self) -> bool {
        if !self.valid { return false; }
        if self.is4 {
            let b = self.As4();
            return b[0] & 0xf0 == 0xe0;
        }
        (self.hi >> 56) as u8 == 0xff
    }

    pub fn IsLinkLocalUnicast(&self) -> bool {
        if !self.valid { return false; }
        if self.is4 {
            let b = self.As4();
            return b[0] == 169 && b[1] == 254;
        }
        (self.hi >> 48) as u16 == 0xfe80
    }

    pub fn IsPrivate(&self) -> bool {
        if !self.valid { return false; }
        if self.is4 {
            let b = self.As4();
            return b[0] == 10
                || (b[0] == 172 && (b[1] & 0xf0) == 16)
                || (b[0] == 192 && b[1] == 168);
        }
        // RFC 4193 fc00::/7
        (self.hi >> 57) as u8 == 0xfc >> 1
    }

    pub fn String(&self) -> string {
        if !self.valid { return "invalid IP".to_string(); }
        if self.is4 {
            let b = self.As4();
            return format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
        }
        let mut out = v6_string(self.hi, self.lo);
        if !self.zone.is_empty() {
            out.push('%');
            out.push_str(&self.zone);
        }
        out
    }

    pub fn Compare(&self, other: &Addr) -> i64 {
        // Go's Compare: invalid < v4 < v6, then numerically, then zone.
        let a_kind = if !self.valid { 0 } else if self.is4 { 1 } else { 2 };
        let b_kind = if !other.valid { 0 } else if other.is4 { 1 } else { 2 };
        if a_kind != b_kind { return (a_kind - b_kind) as i64; }
        if !self.valid { return 0; }
        if self.hi != other.hi { return if self.hi < other.hi { -1 } else { 1 }; }
        if self.lo != other.lo { return if self.lo < other.lo { -1 } else { 1 }; }
        if self.zone == other.zone { 0 }
        else if self.zone < other.zone { -1 } else { 1 }
    }

    pub fn Less(&self, other: &Addr) -> bool { self.Compare(other) < 0 }

    pub fn Next(&self) -> Addr {
        if !self.valid { return self.clone(); }
        let (lo, carry) = self.lo.overflowing_add(1);
        let hi = if carry { self.hi.wrapping_add(1) } else { self.hi };
        if self.is4 {
            if (lo & 0xffff_ffff) == 0 {
                return Addr::default();
            }
            return Addr { hi: 0, lo: lo & 0xffff_ffff, is4: true, zone: String::new(), valid: true };
        }
        if carry && hi == 0 {
            return Addr::default();
        }
        Addr { hi, lo, is4: false, zone: self.zone.clone(), valid: true }
    }

    pub fn Prev(&self) -> Addr {
        if !self.valid { return self.clone(); }
        if self.is4 {
            let v = (self.lo & 0xffff_ffff) as u32;
            if v == 0 { return Addr::default(); }
            let v = v - 1;
            return Addr { hi: 0, lo: v as u64, is4: true, zone: String::new(), valid: true };
        }
        if self.hi == 0 && self.lo == 0 { return Addr::default(); }
        let (lo, borrow) = self.lo.overflowing_sub(1);
        let hi = if borrow { self.hi.wrapping_sub(1) } else { self.hi };
        Addr { hi, lo, is4: false, zone: self.zone.clone(), valid: true }
    }

    pub fn MarshalText(&self) -> (Vec<u8>, error) {
        if !self.valid { return (Vec::new(), nil); }
        (self.String().into_bytes(), nil)
    }
}

pub fn IPv4Unspecified() -> Addr { AddrFrom4([0, 0, 0, 0]) }
pub fn IPv6Unspecified() -> Addr { AddrFrom16([0u8; 16]) }
pub fn IPv6Loopback() -> Addr {
    let mut b = [0u8; 16]; b[15] = 1; AddrFrom16(b)
}

pub fn AddrFrom4(b: [u8; 4]) -> Addr {
    let v = u32::from_be_bytes(b) as u64;
    Addr { hi: 0, lo: v, is4: true, zone: String::new(), valid: true }
}

pub fn AddrFrom16(b: [u8; 16]) -> Addr {
    let mut hi8 = [0u8; 8];
    let mut lo8 = [0u8; 8];
    hi8.copy_from_slice(&b[..8]);
    lo8.copy_from_slice(&b[8..]);
    Addr { hi: u64::from_be_bytes(hi8), lo: u64::from_be_bytes(lo8), is4: false, zone: String::new(), valid: true }
}

pub fn AddrFromSlice(b: &[u8]) -> (Addr, bool) {
    match b.len() {
        4 => {
            let mut a = [0u8; 4]; a.copy_from_slice(b); (AddrFrom4(a), true)
        }
        16 => {
            let mut a = [0u8; 16]; a.copy_from_slice(b); (AddrFrom16(a), true)
        }
        _ => (Addr::default(), false),
    }
}

pub fn ParseAddr(s: &str) -> (Addr, error) {
    // Split off zone
    let (addr_part, zone) = match s.find('%') {
        Some(i) => (&s[..i], s[i + 1..].to_string()),
        None => (s, String::new()),
    };

    if addr_part.contains(':') {
        // IPv6 (+ maybe embedded IPv4 in last 32 bits)
        match parse_v6(addr_part) {
            Ok((hi, lo)) => {
                let mut a = Addr { hi, lo, is4: false, zone: String::new(), valid: true };
                if !zone.is_empty() { a.zone = zone; }
                (a, nil)
            }
            Err(e) => (Addr::default(), New(&format!("ParseAddr({:?}): {}", s, e))),
        }
    } else if addr_part.contains('.') {
        if !zone.is_empty() {
            return (Addr::default(), New(&format!("ParseAddr({:?}): IPv4 addresses can't have a zone", s)));
        }
        match parse_v4(addr_part) {
            Ok(b) => (AddrFrom4(b), nil),
            Err(e) => (Addr::default(), New(&format!("ParseAddr({:?}): {}", s, e))),
        }
    } else {
        (Addr::default(), New(&format!("ParseAddr({:?}): unable to parse IP", s)))
    }
}

pub fn MustParseAddr(s: &str) -> Addr {
    let (a, err) = ParseAddr(s);
    if err != nil { panic!("netip: MustParseAddr({:?}): {}", s, err); }
    a
}

fn parse_v4(s: &str) -> Result<[u8; 4], String> {
    let mut out = [0u8; 4];
    let mut i = 0usize;
    let mut fields = 0usize;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if fields >= 4 { return Err("IPv4 field too long".into()); }
        let start = i;
        while i < bytes.len() && bytes[i] != b'.' {
            if !(b'0'..=b'9').contains(&bytes[i]) {
                return Err(format!("unexpected character {:?}", bytes[i] as char));
            }
            i += 1;
            if i - start > 3 { return Err("IPv4 field has too many digits".into()); }
        }
        if i == start { return Err("IPv4 field is empty".into()); }
        let seg = &s[start..i];
        if seg.len() > 1 && seg.starts_with('0') {
            return Err(format!("IPv4 field has octet with leading zero"));
        }
        let v: u32 = seg.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
        if v > 255 { return Err("IPv4 field has value >255".into()); }
        out[fields] = v as u8;
        fields += 1;
        if i < bytes.len() {
            // expect '.'
            i += 1;
            if i == bytes.len() {
                return Err("trailing dot".into());
            }
        }
    }
    if fields != 4 { return Err(format!("IPv4 address too short")); }
    Ok(out)
}

fn parse_v6(s: &str) -> Result<(u64, u64), String> {
    // Support "::" elision, embedded IPv4 in last 32 bits.
    let mut groups: Vec<u16> = Vec::with_capacity(8);
    let mut tail: Option<Vec<u16>> = None;
    let mut embedded4: Option<[u8; 4]> = None;

    let mut rest = s;
    // Handle leading "::" early
    if rest.starts_with("::") {
        rest = &rest[2..];
        tail = Some(Vec::new());
        if rest.is_empty() { return Ok((0, 0)); }
    }

    loop {
        // find next ':' or end
        let end_colon = rest.find(':');
        let end_dot = rest.find('.');
        match (end_colon, end_dot) {
            (Some(ci), Some(di)) if di < ci => {
                // embedded IPv4 found in this group — actually no colon before the dot
                let v4 = parse_v4(rest)?;
                embedded4 = Some(v4);
                rest = "";
                break;
            }
            (None, Some(_)) => {
                // only dots remain — embedded IPv4
                let v4 = parse_v4(rest)?;
                embedded4 = Some(v4);
                rest = "";
                break;
            }
            (Some(ci), _) => {
                let field = &rest[..ci];
                if field.is_empty() {
                    // "::" in the middle
                    if tail.is_some() {
                        return Err("multiple :: in address".into());
                    }
                    tail = Some(Vec::new());
                    rest = &rest[ci + 1..];
                    if rest.is_empty() { break; }
                    if rest.starts_with(':') {
                        return Err("unexpected : after ::".into());
                    }
                    continue;
                }
                let v = parse_v6_field(field)?;
                match &mut tail {
                    Some(t) => t.push(v),
                    None => groups.push(v),
                }
                rest = &rest[ci + 1..];
                if rest.is_empty() { return Err("trailing colon".into()); }
            }
            (None, None) => {
                if rest.is_empty() { break; }
                let v = parse_v6_field(rest)?;
                match &mut tail {
                    Some(t) => t.push(v),
                    None => groups.push(v),
                }
                rest = "";
                break;
            }
        }
    }

    // Combine groups
    let mut all = [0u16; 8];
    let head_len = groups.len();
    let had_tail = tail.is_some();
    let tail_groups = tail.unwrap_or_default();
    let tail_len = tail_groups.len();
    let has_v4 = embedded4.is_some();
    let mut filled = head_len + tail_len;
    if has_v4 { filled += 2; }
    if filled > 8 { return Err("IPv6 too many groups".into()); }

    for (i, g) in groups.iter().enumerate() { all[i] = *g; }
    if had_tail || has_v4 {
        let mut pos = 8 - tail_len;
        if has_v4 { pos -= 2; }
        for (i, g) in tail_groups.iter().enumerate() { all[pos + i] = *g; }
        if let Some(v4) = embedded4 {
            all[6] = ((v4[0] as u16) << 8) | (v4[1] as u16);
            all[7] = ((v4[2] as u16) << 8) | (v4[3] as u16);
        }
    } else if filled != 8 {
        return Err("IPv6 too few groups".into());
    }

    let hi = ((all[0] as u64) << 48) | ((all[1] as u64) << 32) | ((all[2] as u64) << 16) | (all[3] as u64);
    let lo = ((all[4] as u64) << 48) | ((all[5] as u64) << 32) | ((all[6] as u64) << 16) | (all[7] as u64);
    Ok((hi, lo))
}

fn parse_v6_field(s: &str) -> Result<u16, String> {
    if s.is_empty() || s.len() > 4 { return Err(format!("bad IPv6 field {:?}", s)); }
    let mut v: u32 = 0;
    for c in s.chars() {
        let d = c.to_digit(16).ok_or_else(|| format!("bad IPv6 field {:?}", s))?;
        v = (v << 4) | d;
    }
    Ok(v as u16)
}

fn v6_string(hi: u64, lo: u64) -> string {
    let groups = [
        ((hi >> 48) & 0xffff) as u16,
        ((hi >> 32) & 0xffff) as u16,
        ((hi >> 16) & 0xffff) as u16,
        (hi & 0xffff) as u16,
        ((lo >> 48) & 0xffff) as u16,
        ((lo >> 32) & 0xffff) as u16,
        ((lo >> 16) & 0xffff) as u16,
        (lo & 0xffff) as u16,
    ];

    // Find longest run of zeros (length ≥ 2) for "::" elision.
    let (mut best_start, mut best_len) = (usize::MAX, 0usize);
    let mut i = 0;
    while i < 8 {
        if groups[i] == 0 {
            let mut j = i;
            while j < 8 && groups[j] == 0 { j += 1; }
            let run = j - i;
            if run > best_len {
                best_start = i; best_len = run;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    if best_len < 2 { best_start = usize::MAX; }

    let mut out = String::new();
    let mut i = 0;
    while i < 8 {
        if i == best_start {
            out.push_str("::");
            i += best_len;
            continue;
        }
        if i > 0 && i != best_start && !(best_start != usize::MAX && i == best_start + best_len) {
            out.push(':');
        }
        out.push_str(&format!("{:x}", groups[i]));
        i += 1;
    }
    // Normalise accidental triple-colon
    while out.contains(":::") { out = out.replace(":::", "::"); }
    if out.is_empty() { out = "::".to_string(); }
    out
}

// ── AddrPort ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct AddrPort {
    ip: Addr,
    port: u16,
}

impl AddrPort {
    pub fn Addr(&self) -> Addr { self.ip.clone() }
    pub fn Port(&self) -> u16 { self.port }
    pub fn IsValid(&self) -> bool { self.ip.IsValid() }
    pub fn String(&self) -> string {
        if self.ip.Is6() || self.ip.Is4In6() {
            format!("[{}]:{}", self.ip.String(), self.port)
        } else {
            format!("{}:{}", self.ip.String(), self.port)
        }
    }
}

pub fn AddrPortFrom(ip: Addr, port: u16) -> AddrPort { AddrPort { ip, port } }

pub fn ParseAddrPort(s: &str) -> (AddrPort, error) {
    // Three shapes: "v4:port", "[v6]:port", "[v6%zone]:port"
    if s.starts_with('[') {
        let close = match s.find(']') {
            Some(i) => i,
            None => return (AddrPort::default(), New(&format!("ParseAddrPort({:?}): missing ]", s))),
        };
        let host = &s[1..close];
        let rest = &s[close + 1..];
        if !rest.starts_with(':') {
            return (AddrPort::default(), New(&format!("ParseAddrPort({:?}): missing port", s)));
        }
        let port_s = &rest[1..];
        let (ip, err) = ParseAddr(host);
        if err != nil { return (AddrPort::default(), err); }
        let port: u16 = match port_s.parse() {
            Ok(p) => p,
            Err(_) => return (AddrPort::default(), New(&format!("ParseAddrPort({:?}): bad port", s))),
        };
        return (AddrPort { ip, port }, nil);
    }
    // IPv4 form
    let idx = match s.rfind(':') {
        Some(i) => i,
        None => return (AddrPort::default(), New(&format!("ParseAddrPort({:?}): missing port", s))),
    };
    let host = &s[..idx];
    let port_s = &s[idx + 1..];
    let (ip, err) = ParseAddr(host);
    if err != nil { return (AddrPort::default(), err); }
    if ip.Is6() {
        return (AddrPort::default(), New(&format!("ParseAddrPort({:?}): IPv6 requires brackets", s)));
    }
    let port: u16 = match port_s.parse() {
        Ok(p) => p,
        Err(_) => return (AddrPort::default(), New(&format!("ParseAddrPort({:?}): bad port", s))),
    };
    (AddrPort { ip, port }, nil)
}

pub fn MustParseAddrPort(s: &str) -> AddrPort {
    let (ap, err) = ParseAddrPort(s);
    if err != nil { panic!("netip: MustParseAddrPort({:?}): {}", s, err); }
    ap
}

// ── Prefix (CIDR) ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Prefix {
    ip: Addr,
    bits: i32,
    valid: bool,
}

impl Prefix {
    pub fn Addr(&self) -> Addr { self.ip.clone() }
    pub fn Bits(&self) -> i64 { if self.valid { self.bits as i64 } else { -1 } }
    pub fn IsValid(&self) -> bool { self.valid }

    pub fn String(&self) -> string {
        if !self.valid { return "invalid Prefix".to_string(); }
        format!("{}/{}", self.ip.String(), self.bits)
    }

    pub fn Contains(&self, a: Addr) -> bool {
        if !self.valid || !a.IsValid() { return false; }
        if self.ip.is4 != a.is4 { return false; }
        let bits = self.bits as u32;
        if self.ip.is4 {
            let mask = if bits == 0 { 0u32 } else { !0u32 << (32 - bits) };
            let ap = (a.lo & 0xffff_ffff) as u32;
            let pp = (self.ip.lo & 0xffff_ffff) as u32;
            return ap & mask == pp & mask;
        }
        // IPv6
        let (m_hi, m_lo) = v6_mask(bits);
        (a.hi & m_hi == self.ip.hi & m_hi) && (a.lo & m_lo == self.ip.lo & m_lo)
    }

    pub fn Masked(&self) -> Prefix {
        if !self.valid { return self.clone(); }
        let bits = self.bits as u32;
        let mut ip = self.ip.clone();
        if ip.is4 {
            let mask = if bits == 0 { 0u32 } else { !0u32 << (32 - bits) };
            let v = (ip.lo & 0xffff_ffff) as u32 & mask;
            ip.lo = v as u64;
        } else {
            let (m_hi, m_lo) = v6_mask(bits);
            ip.hi &= m_hi;
            ip.lo &= m_lo;
            ip.zone = String::new();
        }
        Prefix { ip, bits: self.bits, valid: true }
    }
}

fn v6_mask(bits: u32) -> (u64, u64) {
    if bits == 0 { return (0, 0); }
    if bits <= 64 {
        let m = if bits == 64 { !0u64 } else { !0u64 << (64 - bits) };
        (m, 0)
    } else if bits <= 128 {
        let m = if bits == 128 { !0u64 } else { !0u64 << (128 - bits) };
        (!0u64, m)
    } else {
        (!0u64, !0u64)
    }
}

pub fn PrefixFrom(ip: Addr, bits: i64) -> Prefix {
    let max = if ip.is4 { 32 } else { 128 };
    if bits < 0 || bits > max {
        return Prefix { ip, bits: bits as i32, valid: false };
    }
    Prefix { ip, bits: bits as i32, valid: true }
}

pub fn ParsePrefix(s: &str) -> (Prefix, error) {
    let slash = match s.rfind('/') {
        Some(i) => i,
        None => return (Prefix::default(), New(&format!("ParsePrefix({:?}): no / found", s))),
    };
    let (ip, err) = ParseAddr(&s[..slash]);
    if err != nil { return (Prefix::default(), err); }
    let bits_s = &s[slash + 1..];
    let bits: i64 = match bits_s.parse() {
        Ok(b) => b,
        Err(_) => return (Prefix::default(), New(&format!("ParsePrefix({:?}): bad bits", s))),
    };
    let max = if ip.is4 { 32 } else { 128 };
    if bits < 0 || bits > max {
        return (Prefix::default(), New(&format!("ParsePrefix({:?}): prefix length out of range", s)));
    }
    (Prefix { ip, bits: bits as i32, valid: true }, nil)
}

pub fn MustParsePrefix(s: &str) -> Prefix {
    let (p, err) = ParsePrefix(s);
    if err != nil { panic!("netip: MustParsePrefix({:?}): {}", s, err); }
    p
}
