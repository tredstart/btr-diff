use crate::token::Token;

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
