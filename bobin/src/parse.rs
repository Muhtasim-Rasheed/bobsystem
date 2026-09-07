use crate::bobinary::BobinaryFile;

const MAGIC: [u8; 5] = *b"BOBIN";
const VERSION: u16 = 0x0000;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedEof,
    BadMagic([u8; 5]),
    UnsupportedVersion(u16),
    TooManyWords(usize),
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

    fn u16(&mut self) -> Result<u16, ParseError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, ParseError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

impl BobinaryFile {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
        let mut r = Reader::new(bytes);

        let magic: [u8; 5] = r.take(5)?.try_into().unwrap();
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
        let words = r
            .take(word_count as usize * 2)?
            .to_vec()
            .chunks(2)
            .map(|s| u16::from_le_bytes(s.try_into().unwrap()))
            .collect::<Vec<_>>();

        Ok(Self { words })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();

        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(self.words.len() as u32).to_le_bytes());
        for word in &self.words {
            out.extend_from_slice(&word.to_le_bytes());
        }

        out
    }
}
