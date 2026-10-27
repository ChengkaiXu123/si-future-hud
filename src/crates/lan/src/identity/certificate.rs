//! The same P-256 certificate format for each native signing-key store.
use crate::Result;

pub(super) fn certificate(
    public: &[u8],
    device: &str,
    sign: impl FnOnce(&[u8]) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    let algorithm = seq(oid(&[1, 2, 840, 10045, 4, 3, 2]));
    let name = seq(der(
        0x31,
        seq([
            oid(&[2, 5, 4, 3]),
            der(0x0c, format!("Agent HUD {device}").into_bytes()),
        ]
        .concat()),
    ));
    let now = chrono::Utc::now();
    let validity = seq([
        der(
            0x18,
            (now - chrono::Duration::days(1))
                .format("%Y%m%d%H%M%SZ")
                .to_string()
                .into_bytes(),
        ),
        der(
            0x18,
            (now + chrono::Duration::days(365))
                .format("%Y%m%d%H%M%SZ")
                .to_string()
                .into_bytes(),
        ),
    ]
    .concat());
    let public_info = seq([
        seq([
            oid(&[1, 2, 840, 10045, 2, 1]),
            oid(&[1, 2, 840, 10045, 3, 1, 7]),
        ]
        .concat()),
        der(0x03, [vec![0], public.into()].concat()),
    ]
    .concat());
    let mut serial = vec![0u8; 16];
    rustls::crypto::ring::default_provider()
        .secure_random
        .fill(&mut serial)
        .map_err(|_| "LAN serial entropy")?;
    serial[0] = (serial[0] & 0x7f) | 1;
    let constraints = seq([
        oid(&[2, 5, 29, 19]),
        der(0x01, vec![0xff]),
        der(0x04, seq(vec![])),
    ]
    .concat());
    let usage = seq([
        oid(&[2, 5, 29, 15]),
        der(0x01, vec![0xff]),
        der(0x04, der(0x03, vec![7, 0x80])),
    ]
    .concat());
    let extended = seq([
        oid(&[2, 5, 29, 37]),
        der(0x04, seq(oid(&[1, 3, 6, 1, 5, 5, 7, 3, 1]))),
    ]
    .concat());
    let alt = seq([
        oid(&[2, 5, 29, 17]),
        der(
            0x04,
            seq(der(0x82, format!("agenthud-{device}.local").into_bytes())),
        ),
    ]
    .concat());
    let tbs = seq([
        der(0xa0, der(0x02, vec![2])),
        der(0x02, serial),
        algorithm.clone(),
        name.clone(),
        validity,
        name,
        public_info,
        der(0xa3, seq([constraints, usage, extended, alt].concat())),
    ]
    .concat());
    let signature = sign(&tbs)?;
    Ok(seq([
        tbs,
        algorithm,
        der(0x03, [vec![0], signature].concat()),
    ]
    .concat()))
}
fn der(tag: u8, value: Vec<u8>) -> Vec<u8> {
    let length = value.len();
    let mut bytes = vec![tag];
    if length < 128 {
        bytes.push(length as u8);
    } else {
        let raw = length.to_be_bytes();
        let start = raw.iter().position(|b| *b != 0).unwrap_or(raw.len() - 1);
        bytes.push(0x80 | (raw.len() - start) as u8);
        bytes.extend_from_slice(&raw[start..]);
    }
    bytes.extend(value);
    bytes
}
fn seq(value: Vec<u8>) -> Vec<u8> {
    der(0x30, value)
}
fn oid(parts: &[u64]) -> Vec<u8> {
    let mut bytes = vec![(parts[0] * 40 + parts[1]) as u8];
    for part in &parts[2..] {
        let mut value = *part;
        let mut encoded = vec![(value & 0x7f) as u8];
        value >>= 7;
        while value > 0 {
            encoded.insert(0, ((value & 0x7f) | 0x80) as u8);
            value >>= 7;
        }
        bytes.extend(encoded);
    }
    der(0x06, bytes)
}
