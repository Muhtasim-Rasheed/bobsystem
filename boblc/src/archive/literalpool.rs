use std::collections::HashMap;
use std::fmt::Write as _;

pub struct LiteralPool {
    next_id: usize,
    strings: HashMap<String, String>, // content -> label
    ints: HashMap<i64, String>,       // value -> label
    funcs: HashMap<String, String>,   // name -> label
}

impl LiteralPool {
    pub fn new() -> Self {
        LiteralPool {
            next_id: 0,
            strings: HashMap::new(),
            ints: HashMap::new(),
            funcs: HashMap::new(),
        }
    }

    pub fn intern_string(&mut self, s: &str) -> String {
        if let Some(label) = self.strings.get(s) {
            return label.clone();
        }
        let label = format!("__lstr{}", self.next_id);
        self.next_id += 1;
        self.strings.insert(s.to_string(), label.clone());
        label
    }

    pub fn intern_int(&mut self, n: i64) -> String {
        if let Some(label) = self.ints.get(&n) {
            return label.clone();
        }
        let label = format!("__lint{}", self.next_id);
        self.next_id += 1;
        self.ints.insert(n, label.clone());
        label
    }

    pub fn intern_func(&mut self, f: &str) -> String {
        if let Some(label) = self.funcs.get(f) {
            return label.clone();
        }
        let label = format!("__lfunc{}", self.next_id);
        self.next_id += 1;
        self.funcs.insert(f.to_string(), label.clone());
        label
    }

    pub fn emit_data_section(&self, out: &mut String) {
        for (content, label) in &self.strings {
            let escaped = content.replace('\\', "\\\\").replace('"', "\\\"");
            let _ = writeln!(out, "{label}: .string \"{escaped}\"");
        }
        for (value, label) in &self.ints {
            let _ = writeln!(out, "{label}: .fill {value}");
        }
        for (name, label) in &self.funcs {
            let _ = writeln!(out, "{label}: .fill {name}");
        }
    }

    pub fn clear(&mut self) {
        // don't set next_id to 0 otherwise it will collide with previous label names
        self.strings.clear();
        self.ints.clear();
        self.funcs.clear();
    }

    pub fn len(&self) -> usize {
        self.strings.len() + self.ints.len() + self.funcs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn size(&self) -> usize {
        self.strings.keys().map(|s| s.len() + 1).sum::<usize>() + self.ints.len() + self.funcs.len()
    }
}
