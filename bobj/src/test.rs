use crate::{bobject::*, parse::ParseError};

fn sample() -> BobjectFile {
    BobjectFile {
        symbols: vec![
            Symbol {
                name: "main".to_string(),
                offset: 0,
                binding: Binding::Global,
            },
            Symbol {
                name: ".L0".to_string(),
                offset: 5,
                binding: Binding::Local,
            },
        ],
        relocations: vec![
            Relocation {
                site: 1,
                kind: RelocationKind::PcRel9,
                name: ".L0".to_string(),
            },
            Relocation {
                site: 3,
                kind: RelocationKind::PcRel11,
                name: "main".to_string(),
            },
            Relocation {
                site: 6,
                kind: RelocationKind::Abs16,
                name: "main".to_string(),
            },
        ],
        words: vec![0x1226, 0x0A00, 0xC000, 0x0000, 0xF000, 0x0000, 0x0000],
    }
}

fn assert_eq_bobjectfile(a: &BobjectFile, b: &BobjectFile) {
    assert_eq!(a.words, b.words);

    assert_eq!(a.symbols.len(), b.symbols.len());
    for (sa, sb) in a.symbols.iter().zip(&b.symbols) {
        assert_eq!(sa.name, sb.name);
        assert_eq!(sa.offset, sb.offset);
        assert_eq!(sa.binding, sb.binding);
    }

    assert_eq!(a.relocations.len(), b.relocations.len());
    for (ra, rb) in a.relocations.iter().zip(&b.relocations) {
        assert_eq!(ra.site, rb.site);
        assert_eq!(ra.kind, rb.kind);
        assert_eq!(ra.name, rb.name);
    }
}

#[test]
fn round_trip() {
    let original = sample();
    let bytes = original.to_bytes();
    let parsed = BobjectFile::from_bytes(&bytes).expect("parse should succeed");
    assert_eq_bobjectfile(&original, &parsed);
}

#[test]
fn empty_file_round_trips() {
    let original = BobjectFile {
        symbols: vec![],
        relocations: vec![],
        words: vec![],
    };
    let bytes = original.to_bytes();
    let parsed = BobjectFile::from_bytes(&bytes).expect("parse should succeed");
    assert_eq_bobjectfile(&original, &parsed);
}

#[test]
fn rejects_bad_magic() {
    let mut bytes = sample().to_bytes();
    bytes[0] = b'X';
    assert!(matches!(
        BobjectFile::from_bytes(&bytes),
        Err(ParseError::BadMagic(_))
    ));
}

#[test]
fn rejects_truncated_file() {
    let bytes = sample().to_bytes();
    let truncated = &bytes[..bytes.len() - 3];
    assert!(matches!(
        BobjectFile::from_bytes(truncated),
        Err(ParseError::UnexpectedEof)
    ));
}

#[test]
fn rejects_unsupported_version() {
    let mut bytes = sample().to_bytes();
    bytes[4] = 0xFF;
    bytes[5] = 0xFF;
    assert!(matches!(
        BobjectFile::from_bytes(&bytes),
        Err(ParseError::UnsupportedVersion(0xFFFF))
    ));
}

#[test]
fn rejects_file_with_extra_words() {
    let mut bytes = sample().to_bytes();
    bytes[6] = 0x01;
    bytes[7] = 0x00;
    bytes[8] = 0x01;
    bytes[9] = 0x00;
    assert!(matches!(
        BobjectFile::from_bytes(&bytes),
        Err(ParseError::TooManyWords(1)),
    ));
}
