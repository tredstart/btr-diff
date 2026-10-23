use crate::token::Token;

// --- Line-level types (new btr format) ---

#[derive(Debug)]
pub enum Segment {
    Equal(String),
    Change { old: String, new: String },
}

#[derive(Debug)]
pub enum LineDiffOp {
    Equal(u32),
    Delete(u32),
    Insert { pos: u32, content: String },
    Modify { line: u32, segments: Vec<Segment> },
    Move { from_start: u32, from_end: u32, to_after: u32 },
}

// --- Word-level types (existing) ---

pub enum DiffOp {
    Equal(Token),
    Insert(Token),
    Delete(Token),
    Replace(Token, Token),
}

fn lcs(a: &[Token], b: &[Token]) -> Vec<u32> {
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

pub fn diff(a: &[Token], b: &[Token]) -> Vec<DiffOp> {
    let w = b.len() + 1;
    let table = lcs(a, b);
    let (mut i, mut j) = (0, 0);
    let mut raw: Vec<DiffOp> = Vec::new();

    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i].hash == b[j].hash {
            raw.push(DiffOp::Equal(a[i].clone()));
            i += 1;
            j += 1;
        } else if j >= b.len() || (i < a.len() && table[(i + 1) * w + j] >= table[i * w + j + 1]) {
            raw.push(DiffOp::Delete(a[i].clone()));
            i += 1;
        } else {
            raw.push(DiffOp::Insert(b[j].clone()));
            j += 1;
        }
    }

    pair_replacements(raw)
}

fn pair_replacements(raw: Vec<DiffOp>) -> Vec<DiffOp> {
    let mut ops: Vec<DiffOp> = Vec::new();
    let mut deletes: Vec<Token> = Vec::new();
    let mut inserts: Vec<Token> = Vec::new();

    for op in raw {
        match op {
            DiffOp::Delete(t) => deletes.push(t),
            DiffOp::Insert(t) => inserts.push(t),
            other => {
                flush(&mut deletes, &mut inserts, &mut ops);
                ops.push(other);
            }
        }
    }
    flush(&mut deletes, &mut inserts, &mut ops);
    ops
}

fn flush(deletes: &mut Vec<Token>, inserts: &mut Vec<Token>, ops: &mut Vec<DiffOp>) {
    let pairs = deletes.len().min(inserts.len());
    for _ in 0..pairs {
        ops.push(DiffOp::Replace(deletes.remove(0), inserts.remove(0)));
    }
    for d in deletes.drain(..) {
        ops.push(DiffOp::Delete(d));
    }
    for i in inserts.drain(..) {
        ops.push(DiffOp::Insert(i));
    }
}

// --- Prepared lines: tokenize once, reuse everywhere ---

use crate::token;

pub struct PreparedLine {
    pub tokens: Vec<Token>,
    pub hash: u64,
}

pub fn prepare_lines(source: &str) -> Vec<PreparedLine> {
    source
        .lines()
        .map(|line| {
            let tokens = token::tokenize(line);
            let hash = tokens.iter().fold(0u64, |h, t| {
                h.wrapping_mul(1000003).wrapping_add(t.hash)
            });
            PreparedLine { tokens, hash }
        })
        .collect()
}

// --- Word-level diff producing Segments (for use within a single line) ---

fn word_diff(old_tokens: &[Token], new_tokens: &[Token]) -> Vec<Segment> {
    let ops = diff(old_tokens, new_tokens);

    let mut segments: Vec<Segment> = Vec::new();
    for op in ops {
        match op {
            DiffOp::Equal(t) => match segments.last_mut() {
                Some(Segment::Equal(s)) => s.push_str(&t.seq),
                _ => segments.push(Segment::Equal(t.seq)),
            },
            DiffOp::Replace(old, new) => match segments.last_mut() {
                Some(Segment::Change { old: o, new: n }) => {
                    o.push_str(&old.seq);
                    n.push_str(&new.seq);
                }
                _ => segments.push(Segment::Change {
                    old: old.seq,
                    new: new.seq,
                }),
            },
            DiffOp::Delete(t) => match segments.last_mut() {
                Some(Segment::Change { old: o, .. }) => o.push_str(&t.seq),
                _ => segments.push(Segment::Change {
                    old: t.seq,
                    new: String::new(),
                }),
            },
            DiffOp::Insert(t) => match segments.last_mut() {
                Some(Segment::Change { new: n, .. }) => n.push_str(&t.seq),
                _ => segments.push(Segment::Change {
                    old: String::new(),
                    new: t.seq,
                }),
            },
        }
    }
    segments
}

// --- Line-level diff ---

fn lcs_lines(hashes_a: &[u64], hashes_b: &[u64]) -> Vec<u32> {
    let w = hashes_b.len() + 1;
    let mut table = vec![0u32; (hashes_a.len() + 1) * w];
    for i in (0..hashes_a.len()).rev() {
        for j in (0..hashes_b.len()).rev() {
            table[i * w + j] = if hashes_a[i] == hashes_b[j] {
                table[(i + 1) * w + j + 1] + 1
            } else {
                table[(i + 1) * w + j].max(table[i * w + j + 1])
            };
        }
    }
    table
}

enum RawLineOp {
    Equal(usize, usize),
    Delete(usize),
    Insert(usize),
}

fn trace_lines(hashes_a: &[u64], hashes_b: &[u64], table: &[u32]) -> Vec<RawLineOp> {
    let w = hashes_b.len() + 1;
    let (mut i, mut j) = (0, 0);
    let mut ops = Vec::new();
    while i < hashes_a.len() || j < hashes_b.len() {
        if i < hashes_a.len() && j < hashes_b.len() && hashes_a[i] == hashes_b[j] {
            ops.push(RawLineOp::Equal(i, j));
            i += 1;
            j += 1;
        } else if j >= hashes_b.len()
            || (i < hashes_a.len() && table[(i + 1) * w + j] >= table[i * w + j + 1])
        {
            ops.push(RawLineOp::Delete(i));
            i += 1;
        } else {
            ops.push(RawLineOp::Insert(j));
            j += 1;
        }
    }
    ops
}

fn pair_line_ops(
    raw: Vec<RawLineOp>,
    prep_a: &[PreparedLine],
    prep_b: &[PreparedLine],
) -> Vec<LineDiffOp> {
    let mut ops: Vec<LineDiffOp> = Vec::new();
    let mut deletes: Vec<usize> = Vec::new();
    let mut inserts: Vec<usize> = Vec::new();
    let mut last_a: u32 = 0;

    for op in raw {
        match op {
            RawLineOp::Equal(i, _j) => {
                flush_line_ops(&mut deletes, &mut inserts, &mut ops, prep_a, prep_b, last_a);
                let line = (i as u32) + 1;
                ops.push(LineDiffOp::Equal(line));
                last_a = line;
            }
            RawLineOp::Delete(i) => {
                deletes.push(i);
                last_a = (i as u32) + 1;
            }
            RawLineOp::Insert(j) => {
                inserts.push(j);
            }
        }
    }
    flush_line_ops(&mut deletes, &mut inserts, &mut ops, prep_a, prep_b, last_a);
    ops
}

fn flush_line_ops(
    deletes: &mut Vec<usize>,
    inserts: &mut Vec<usize>,
    ops: &mut Vec<LineDiffOp>,
    prep_a: &[PreparedLine],
    prep_b: &[PreparedLine],
    last_a: u32,
) {
    let pairs = deletes.len().min(inserts.len());
    for k in 0..pairs {
        let ai = deletes[k];
        let bi = inserts[k];
        let segs = word_diff(&prep_a[ai].tokens, &prep_b[bi].tokens);
        ops.push(LineDiffOp::Modify {
            line: (ai as u32) + 1,
            segments: segs,
        });
    }
    for &d in &deletes[pairs..] {
        ops.push(LineDiffOp::Delete((d as u32) + 1));
    }
    for &i in &inserts[pairs..] {
        let content: String = prep_b[i].tokens.iter().map(|t| t.seq.as_str()).collect();
        ops.push(LineDiffOp::Insert {
            pos: last_a,
            content,
        });
    }
    deletes.clear();
    inserts.clear();
}

pub fn diff_lines(prep_a: &[PreparedLine], prep_b: &[PreparedLine]) -> Vec<LineDiffOp> {
    let hashes_a: Vec<u64> = prep_a.iter().map(|p| p.hash).collect();
    let hashes_b: Vec<u64> = prep_b.iter().map(|p| p.hash).collect();

    let table = lcs_lines(&hashes_a, &hashes_b);
    let raw = trace_lines(&hashes_a, &hashes_b, &table);
    pair_line_ops(raw, prep_a, prep_b)
}
