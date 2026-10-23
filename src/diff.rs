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
    Insert { pos: u32, content: String, line_b: u32 },
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

pub fn word_diff(old_tokens: &[Token], new_tokens: &[Token]) -> Vec<Segment> {
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
            line_b: (i as u32) + 1,
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
    let paired = pair_line_ops(raw, prep_a, prep_b);
    detect_moves(paired, prep_a, prep_b)
}

// --- Move detection (exact match via HashMap) ---

use std::collections::HashMap;

struct Block {
    start_idx: usize,
    end_idx: usize,
    content_hash: u64,
    line_start: u32,
}

fn detect_moves(
    ops: Vec<LineDiffOp>,
    prep_a: &[PreparedLine],
    prep_b: &[PreparedLine],
) -> Vec<LineDiffOp> {
    let mut delete_blocks: Vec<Block> = Vec::new();
    let mut insert_blocks: Vec<Block> = Vec::new();

    let mut i = 0;
    while i < ops.len() {
        match &ops[i] {
            LineDiffOp::Delete(line) => {
                let start = i;
                let line_start = *line;
                let mut h = prep_a[(*line - 1) as usize].hash;
                i += 1;
                while i < ops.len() {
                    if let LineDiffOp::Delete(l) = &ops[i] {
                        h = h.wrapping_mul(1000003)
                            .wrapping_add(prep_a[(*l - 1) as usize].hash);
                        i += 1;
                    } else {
                        break;
                    }
                }
                delete_blocks.push(Block {
                    start_idx: start,
                    end_idx: i - 1,
                    content_hash: h,
                    line_start,
                });
            }
            LineDiffOp::Insert { line_b, pos, .. } => {
                let start = i;
                let pos_val = *pos;
                let mut h = prep_b[(*line_b - 1) as usize].hash;
                i += 1;
                while i < ops.len() {
                    if let LineDiffOp::Insert { line_b: lb, .. } = &ops[i] {
                        h = h.wrapping_mul(1000003)
                            .wrapping_add(prep_b[(*lb - 1) as usize].hash);
                        i += 1;
                    } else {
                        break;
                    }
                }
                insert_blocks.push(Block {
                    start_idx: start,
                    end_idx: i - 1,
                    content_hash: h,
                    line_start: pos_val,
                });
            }
            _ => {
                i += 1;
            }
        }
    }

    if delete_blocks.is_empty() || insert_blocks.is_empty() {
        return ops;
    }

    let mut insert_map: HashMap<(u64, usize), usize> = HashMap::new();
    for (idx, ib) in insert_blocks.iter().enumerate() {
        let len = ib.end_idx - ib.start_idx + 1;
        insert_map.entry((ib.content_hash, len)).or_insert(idx);
    }

    let mut skip = vec![false; ops.len()];
    let mut moves: Vec<(usize, LineDiffOp)> = Vec::new();
    let mut used_inserts = vec![false; insert_blocks.len()];

    for db in &delete_blocks {
        let len = db.end_idx - db.start_idx + 1;
        if let Some(&ii) = insert_map.get(&(db.content_hash, len)) {
            if !used_inserts[ii] {
                used_inserts[ii] = true;
                let ib = &insert_blocks[ii];
                for idx in db.start_idx..=db.end_idx {
                    skip[idx] = true;
                }
                for idx in ib.start_idx..=ib.end_idx {
                    skip[idx] = true;
                }
                moves.push((
                    db.start_idx,
                    LineDiffOp::Move {
                        from_start: db.line_start,
                        from_end: db.line_start + (len as u32) - 1,
                        to_after: ib.line_start,
                    },
                ));
            }
        }
    }

    if moves.is_empty() {
        return ops;
    }

    moves.sort_by_key(|m| m.0);
    let mut result: Vec<LineDiffOp> = Vec::new();
    let mi = 0;
    for (idx, op) in ops.into_iter().enumerate() {
        while mi < moves.len() && moves[mi].0 == idx {
            let (_, mov) = moves.remove(mi);
            result.push(mov);
        }
        if !skip[idx] {
            result.push(op);
        }
    }
    result
}
