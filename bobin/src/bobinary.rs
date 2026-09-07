#[derive(Debug)]
pub struct BobinaryFile {
    pub words: Vec<u16>,
}

impl BobinaryFile {
    pub fn word_count(&self) -> usize {
        self.words.len()
    }
}
