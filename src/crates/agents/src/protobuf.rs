//! Bounded protobuf wire decoding shared by original local-provider readers.
//! Unknown fields are skipped; wrong wire types and truncated/overflowing values
//! reject the containing record rather than guessing counters.
// A malformed message has nothing more to say than that it is malformed.
#![allow(clippy::result_unit_err)]
#[derive(Clone, Copy)]
pub struct Field<'a> {
    pub number: u32,
    pub wire: u8,
    data: &'a [u8],
    value: u64,
}
impl<'a> Field<'a> {
    pub fn bytes(self) -> Result<&'a [u8], ()> {
        if self.wire == 2 {
            Ok(self.data)
        } else {
            Err(())
        }
    }
    pub fn integer(self) -> Result<u64, ()> {
        if self.wire == 0 {
            Ok(self.value)
        } else {
            Err(())
        }
    }
    pub fn string(self) -> Result<Option<String>, ()> {
        let s = std::str::from_utf8(self.bytes()?).map_err(|_| ())?;
        Ok((!s.trim().is_empty()).then(|| s.to_owned()))
    }
    pub fn counter(self) -> Result<u64, ()> {
        let n = self.integer()?;
        if n <= i64::MAX as u64 {
            Ok(n)
        } else {
            Err(())
        }
    }
}
fn varint(bytes: &[u8], at: &mut usize) -> Result<u64, ()> {
    let mut n = 0;
    for shift in 0..10 {
        let b = *bytes.get(*at).ok_or(())?;
        *at += 1;
        if shift == 9 && b > 1 {
            return Err(());
        }
        n |= ((b & 127) as u64) << (shift * 7);
        if b & 128 == 0 {
            return Ok(n);
        }
    }
    Err(())
}
pub fn fields<'a>(
    bytes: &'a [u8],
    mut visit: impl FnMut(Field<'a>) -> Result<(), ()>,
) -> Result<(), ()> {
    let mut at = 0;
    while at < bytes.len() {
        let tag = varint(bytes, &mut at)?;
        let number = tag >> 3;
        if number == 0 || number > 536_870_911 {
            return Err(());
        }
        let wire = (tag & 7) as u8;
        let mut field = Field {
            number: number as u32,
            wire,
            data: &[],
            value: 0,
        };
        if wire == 0 {
            field.value = varint(bytes, &mut at)?
        } else {
            let size = match wire {
                1 => 8,
                5 => 4,
                2 => usize::try_from(varint(bytes, &mut at)?).map_err(|_| ())?,
                _ => return Err(()),
            };
            let end = at.checked_add(size).ok_or(())?;
            field.data = bytes.get(at..end).ok_or(())?;
            at = end;
        }
        visit(field)?;
    }
    Ok(())
}
