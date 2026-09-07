use crate::bobject::*;

const MAGIC: [u8; 4] = *b"BOBJ";
const VERSION: u16 = 0x0000;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedEof,
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    TooManyWords(usize),
    InvalidBinding(u8),
    InvalidRelocKind(u8),
    InvalidUtf8Name,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedEof => write!(f, "unexpected end of file"),
            ParseError::BadMagic(got) => {
                write!(f, "bad magic bytes: expected {MAGIC:?}, got {got:?}")
            }
            ParseError::UnsupportedVersion(v) => write!(f, "unsupported version: {v}"),
            ParseError::TooManyWords(extra) => {
                write!(f, "too many words: {extra} extra words")
            }
            ParseError::InvalidBinding(b) => write!(f, "invalid symbol binding byte: {b}"),
            ParseError::InvalidRelocKind(k) => write!(f, "invalid relocation kind byte: {k}"),
            ParseError::InvalidUtf8Name => write!(f, "symbol/relocation name is not valid UTF-8"),
        }
    }
}

impl std::error::Error for ParseError {}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes, pos: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], ParseError> {
        let end = self.pos.checked_add(len).ok_or(ParseError::UnexpectedEof)?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(ParseError::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, ParseError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ParseError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, ParseError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn name(&mut self) -> Result<String, ParseError> {
        let len = self.u8()? as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ParseError::InvalidUtf8Name)
    }
}

impl BobjectFile {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
        let mut r = Reader::new(bytes);

        let magic: [u8; 4] = r.take(4)?.try_into().unwrap();
        if magic != MAGIC {
            return Err(ParseError::BadMagic(magic));
        }

        let version = r.u16()?;
        if version != VERSION {
            return Err(ParseError::UnsupportedVersion(version));
        }
        let word_count = r.u32()?;
        if word_count > 0x10000 {
            return Err(ParseError::TooManyWords(word_count as usize - 0x10000));
        }
        let sym_count = r.u16()?;
        let reloc_count = r.u16()?;

        let mut symbols = Vec::with_capacity(sym_count as usize);
        for _ in 0..sym_count {
            let name = r.name()?;
            let offset = r.u16()?;
            let binding = match r.u8()? {
                0 => Binding::Local,
                1 => Binding::Global,
                b => return Err(ParseError::InvalidBinding(b)),
            };
            symbols.push(Symbol {
                name,
                offset,
                binding,
            });
        }

        let mut relocations = Vec::with_capacity(reloc_count as usize);
        for _ in 0..reloc_count {
            let site = r.u16()?;
            let kind = match r.u8()? {
                0 => RelocationKind::PcRel9,
                1 => RelocationKind::PcRel11,
                2 => RelocationKind::Abs16,
                k => return Err(ParseError::InvalidRelocKind(k)),
            };
            let name = r.name()?;
            relocations.push(Relocation { site, kind, name });
        }

        let mut words = Vec::with_capacity(word_count as usize);
        for _ in 0..word_count {
            words.push(r.u16()?);
        }

        Ok(BobjectFile {
            symbols,
            relocations,
            words,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();

        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(self.words.len() as u32).to_le_bytes());
        out.extend_from_slice(&(self.symbols.len() as u16).to_le_bytes());
        out.extend_from_slice(&(self.relocations.len() as u16).to_le_bytes());

        for sym in &self.symbols {
            debug_assert!(
                sym.name.len() <= u8::MAX as usize,
                "symbol name too long: {}",
                sym.name
            );
            out.push(sym.name.len() as u8);
            out.extend_from_slice(sym.name.as_bytes());
            out.extend_from_slice(&sym.offset.to_le_bytes());
            out.push(match sym.binding {
                Binding::Local => 0,
                Binding::Global => 1,
            });
        }

        for reloc in &self.relocations {
            out.extend_from_slice(&reloc.site.to_le_bytes());
            out.push(match reloc.kind {
                RelocationKind::PcRel9 => 0,
                RelocationKind::PcRel11 => 1,
                RelocationKind::Abs16 => 2,
            });
            debug_assert!(
                reloc.name.len() <= u8::MAX as usize,
                "relocation symbol name too long: {}",
                reloc.name
            );
            out.push(reloc.name.len() as u8);
            out.extend_from_slice(reloc.name.as_bytes());
        }

        for word in &self.words {
            out.extend_from_slice(&word.to_le_bytes());
        }

        out
    }
}
