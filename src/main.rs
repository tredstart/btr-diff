use clap::{Arg, Command};

#[derive(Debug, Clone)]
struct CharSeq {
    seq: String,
    hash: u64,
    line: u128,
}

impl CharSeq {
    fn new() -> Self {
        CharSeq {
            seq: "".to_string(),
            hash: 0,
            line: 0,
        }
    }
    fn empty(self: &Self) -> bool {
        self.hash == 0
    }

    fn clear(self: &mut Self) {
        self.seq = "".to_string();
        self.hash = 0;
        self.line = 0;
    }
}

fn main() {
    let matches = Command::new("btr-diff")
        .arg(Arg::new("f1").required(true))
        .arg(Arg::new("f2").required(true))
        .get_matches();
    let f1 = matches.get_one::<String>("f1").expect("Missing argument");
    let f2 = matches.get_one::<String>("f2").expect("Missing argument");

    std::thread::scope(|s| {
        let handles = [f1, f2].map(|f| {
            s.spawn(move || {
                let mut collection = Vec::new();
                let file = std::fs::read_to_string(f).expect("Cannot read the file!");
                let mut word = CharSeq::new();
                let mut whitespace = CharSeq::new();
                for b in file.bytes() {
                    if b.is_ascii_whitespace() {
                        switch(&mut whitespace, &mut word, b, &mut collection);
                    } else {
                        switch(&mut word, &mut whitespace, b, &mut collection);
                    }
                }

                if word.empty() {
                    collection.push(whitespace.clone());
                } else {
                    collection.push(word.clone());
                }
                collection
            })
        });
        let res: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().expect("Thread failed?"))
            .collect();
        let [a, b] = &res[0..2] else {
            panic!("Collected too many diffs for some reason, skipping for now...");
        };

        let (deletions, insertions) = diff(a, b);

        println!("insertions: ");
        for i in insertions {
            print!("{}", i.seq);
        }
        println!();
        println!("deletions: ");
        for d in deletions {
            print!("{}", d.seq);
        }
        println!();
    });
}

fn switch(a: &mut CharSeq, b: &mut CharSeq, byte: u8, collection: &mut Vec<CharSeq>) {
    a.seq.push(byte.into());
    a.hash = roll(a.hash, byte);
    if !b.empty() {
        collection.push(b.clone());
        b.clear();
    }
}

fn roll(h: u64, byte: u8) -> u64 {
    h.wrapping_mul(31).wrapping_add(byte as u64)
}

fn lcs(a: &[CharSeq], b: &[CharSeq]) -> Vec<u32> {
    let w = b.len() + 1;
    let mut table = vec![0u32; (a.len() + 1) * w];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            table[i * w + j] = if a[i].hash == b[j].hash {
                table[(i + 1) * w + j + 1] + 1
            } else {
                table[(i + 1) * w + j].max(table[i * w + j + 1])
            };
        }
    }
    table
}

fn diff(a: &[CharSeq], b: &[CharSeq]) -> (Vec<CharSeq>, Vec<CharSeq>) {
    let w = b.len() + 1;
    let table = lcs(a, b);
    let (mut i, mut j) = (0, 0);
    let (mut deletions, mut insertions) = (Vec::new(), Vec::new());
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i].hash == b[j].hash {
            i += 1;
            j += 1;
        } else if j >= b.len() || (i < a.len() && table[(i + 1) * w + j] >= table[i * w + j + 1]) {
            deletions.push(a[i].clone());
            i += 1;
        } else {
            insertions.push(b[j].clone());
            j += 1;
        }
    }
    (deletions, insertions)
}
