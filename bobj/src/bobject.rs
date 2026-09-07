#[derive(Debug)]
pub struct BobjectFile {
    pub symbols: Vec<Symbol>,
    pub relocations: Vec<Relocation>,
    pub words: Vec<u16>,
}

impl BobjectFile {
    pub fn word_count(&self) -> u16 {
        self.words.len() as u16
    }

    pub fn sym_count(&self) -> u16 {
        self.symbols.len() as u16
    }

    pub fn reloc_count(&self) -> u16 {
        self.relocations.len() as u16
    }

    pub fn find_symbol(&self, name: &str) -> Option<&Symbol> {
        for symbol in &self.symbols {
            if symbol.name == name {
                return Some(symbol);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub offset: u16,
    pub binding: Binding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    Local,
    Global,
}

#[derive(Debug, Clone)]
pub struct Relocation {
    pub site: u16,
    pub kind: RelocationKind,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocationKind {
    PcRel9,
    PcRel11,
    Abs16,
}
