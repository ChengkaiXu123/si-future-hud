//! Original Antigravity 2.19.1 embedded descriptor fields. Adapted from the
//! MIT CodexBar reader credited in the original project's third-party notices.
use crate::protobuf::{fields, Field};

#[derive(Default, Clone, PartialEq)]
pub struct Usage {
    pub model: Option<u64>,
    pub input: u64,
    pub output: u64,
    pub read: u64,
    pub write: u64,
    pub thinking: u64,
    pub response: u64,
    pub response_id: Option<String>,
    pub message: Option<String>,
    invalid_message: bool,
}

#[derive(Default, Clone, PartialEq)]
pub struct Turn {
    pub usage: Option<Usage>,
    pub at: Option<i64>,
    pub model: Option<String>,
    pub label: Option<String>,
    pub requested: Option<u64>,
    pub execution: Option<String>,
}

#[derive(Default)]
struct Timestamp {
    seconds: Option<u64>,
    nanos: u64,
}

impl Timestamp {
    fn read(&mut self, b: &[u8]) -> Result<(), ()> {
        fields(b, |f| {
            match f.number {
                1 => {
                    let n = f.integer()?;
                    if n == 0 || n > 253_402_300_799 {
                        return Err(());
                    }
                    self.seconds = Some(n)
                }
                2 => {
                    let n = f.integer()?;
                    if n > 999_999_999 {
                        return Err(());
                    }
                    self.nanos = n
                }
                _ => (),
            }
            Ok(())
        })
    }
    fn ms(&self) -> Option<i64> {
        self.seconds
            .map(|s| (s * 1000 + self.nanos / 1_000_000) as i64)
    }
}

fn auxiliary(f: Field<'_>, value: &mut Option<String>, invalid: &mut bool) {
    match f.string() {
        Ok(s) if !*invalid => *value = s,
        Err(()) => {
            *value = None;
            *invalid = true
        }
        _ => (),
    }
}

pub fn turn(bytes: &[u8]) -> Option<Turn> {
    let mut t = Turn::default();
    let mut time = Timestamp::default();
    let mut found = false;
    fields(bytes, |f| {
        match f.number {
            1 => {
                found = true;
                fields(f.bytes()?, |c| {
                    match c.number {
                        3 => t.requested = Some(c.counter()?),
                        4 => {
                            let u = t.usage.get_or_insert_with(Usage::default);
                            fields(c.bytes()?, |v| {
                                match v.number {
                                    1 => u.model = Some(v.counter()?),
                                    2 => u.input = v.counter()?,
                                    3 => u.output = v.counter()?,
                                    4 => u.write = v.counter()?,
                                    5 => u.read = v.counter()?,
                                    7 => auxiliary(v, &mut u.message, &mut u.invalid_message),
                                    9 => u.thinking = v.counter()?,
                                    10 => u.response = v.counter()?,
                                    11 => u.response_id = v.string()?,
                                    _ => (),
                                }
                                Ok(())
                            })?
                        }
                        9 => fields(c.bytes()?, |s| {
                            if s.number == 4 {
                                time.read(s.bytes()?)?
                            }
                            Ok(())
                        })?,
                        19 => t.model = c.string()?,
                        21 => t.label = c.string()?,
                        _ => (),
                    }
                    Ok(())
                })?
            }
            4 => t.execution = f.string()?,
            _ => (),
        }
        Ok(())
    })
    .ok()?;
    t.at = time.ms();
    found.then_some(t)
}

#[derive(Default)]
pub struct Step {
    pub execution: Option<String>,
    pub message: Option<String>,
    pub at: Option<i64>,
}

pub fn step(bytes: &[u8]) -> Option<Step> {
    let mut s = Step::default();
    let mut time = Timestamp::default();
    let mut invalid = false;
    let mut valid_time = true;
    fields(bytes, |f| {
        match f.number {
            1 => {
                if f.bytes().and_then(|b| time.read(b)).is_err() {
                    valid_time = false
                }
            }
            9 => {
                if f.bytes()
                    .and_then(|b| {
                        fields(b, |f| {
                            if f.number == 7 {
                                auxiliary(f, &mut s.message, &mut invalid)
                            }
                            Ok(())
                        })
                    })
                    .is_err()
                {
                    s.message = None;
                    invalid = true
                }
            }
            12 => s.execution = f.string()?,
            _ => (),
        }
        Ok(())
    })
    .ok()?;
    s.at = valid_time.then(|| time.ms()).flatten();
    Some(s)
}
