#[derive(Debug, Clone)]
pub struct Token {
    pub seq: String,
    pub hash: u64,
    pub line: u32,
}

impl Token {
    pub fn new() -> Self {
        Token {
            seq: String::new(),
            hash: 0,
            line: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.hash == 0
    }

    pub fn clear(&mut self) {
        self.seq = String::new();
        self.hash = 0;
        self.line = 0;
    }
}

fn roll(h: u64, byte: u8) -> u64 {
    h.wrapping_mul(31).wrapping_add(byte as u64)
}

fn switch(a: &mut Token, b: &mut Token, byte: u8, collection: &mut Vec<Token>) {
    a.seq.push(byte.into());
    a.hash = roll(a.hash, byte);
    if !b.is_empty() {
        collection.push(b.clone());
        b.clear();
    }
}

pub fn tokenize(source: &str) -> Vec<Token> {
    let mut collection = Vec::new();
    let mut word = Token::new();
    let mut whitespace = Token::new();
    let mut line: u32 = 1;
    for b in source.bytes() {
        if b.is_ascii_whitespace() {
            if whitespace.is_empty() {
                whitespace.line = line;
            }
            switch(&mut whitespace, &mut word, b, &mut collection);
        } else {
            if word.is_empty() {
                word.line = line;
            }
            switch(&mut word, &mut whitespace, b, &mut collection);
        }
        if b == b'\n' {
            line += 1;
        }
    }
    if word.is_empty() {
        collection.push(whitespace.clone());
    } else {
        collection.push(word.clone());
    }
    collection
}
