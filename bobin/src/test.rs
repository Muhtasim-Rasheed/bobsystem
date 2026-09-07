use crate::{bobinary::BobinaryFile, parse::ParseError};

fn sample() -> BobinaryFile {
    BobinaryFile {
        words: vec![0x4002, 0xf200, 0xf000, 0x0004, 0x0062, 0x006f, 0x0062],
    }
}

fn assert_eq_bobinaryfile(a: &BobinaryFile, b: &BobinaryFile) {
    assert_eq!(a.words, b.words);
}

#[test]
fn round_trip() {
    let original = sample();
    let bytes = original.to_bytes();
    let parsed = BobinaryFile::from_bytes(&bytes).expect("parse should succeed");
    assert_eq_bobinaryfile(&original, &parsed);
}

#[test]
fn empty_file_round_trips() {
    let original = BobinaryFile { words: vec![] };
    let bytes = original.to_bytes();
    let parsed = BobinaryFile::from_bytes(&bytes).expect("parse should succeed");
    assert_eq_bobinaryfile(&original, &parsed);
}

#[test]
fn rejects_bad_magic() {
    let mut bytes = sample().to_bytes();
    bytes[0] = b'X';
    assert!(matches!(
        BobinaryFile::from_bytes(&bytes),
        Err(ParseError::BadMagic(_))
    ));
}

#[test]
fn rejects_truncated_file() {
    let bytes = sample().to_bytes();
    let truncated = &bytes[..bytes.len() - 3];
    assert!(matches!(
        BobinaryFile::from_bytes(truncated),
        Err(ParseError::UnexpectedEof)
    ));
}

#[test]
fn rejects_unsupported_version() {
    let mut bytes = sample().to_bytes();
    bytes[5] = 0xFF;
    bytes[6] = 0xFF;
    assert!(matches!(
        BobinaryFile::from_bytes(&bytes),
        Err(ParseError::UnsupportedVersion(0xFFFF))
    ));
}

#[test]
fn rejects_file_with_extra_words() {
    let mut bytes = sample().to_bytes();
    bytes[7] = 0x01;
    bytes[8] = 0x00;
    bytes[9] = 0x01;
    bytes[10] = 0x00;
    assert!(matches!(
        BobinaryFile::from_bytes(&bytes),
        Err(ParseError::TooManyWords(1)),
    ));
}
