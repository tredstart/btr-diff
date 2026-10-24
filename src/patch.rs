use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum PatchSegment {
    Equal(String),
    Change { old: String, new: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatchOp {
    Modify { line: u32, segments: Vec<PatchSegment> },
    Insert { pos: u32, content: String },
    Delete { start: u32, end: u32 },
    Move { from_start: u32, from_end: u32, to_after: u32 },
}

#[derive(Debug)]
pub struct BtrPatch {
    pub source: Option<String>,
    pub target: Option<String>,
    pub ops: Vec<PatchOp>,
}

#[derive(Debug)]
pub enum PatchError {
    InvalidFormat(String),
    InvalidLineNumber(String),
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatchError::InvalidFormat(msg) => write!(f, "invalid patch format: {msg}"),
            PatchError::InvalidLineNumber(msg) => write!(f, "invalid line number: {msg}"),
        }
    }
}

impl std::error::Error for PatchError {}

// --- Parsing ---

pub fn parse(input: &str) -> Result<BtrPatch, PatchError> {
    let mut lines = input.lines();

    let header = lines
        .next()
        .ok_or_else(|| PatchError::InvalidFormat("empty input".into()))?;
    if header.trim() != "btr/1" {
        return Err(PatchError::InvalidFormat(format!(
            "expected 'btr/1', got '{header}'"
        )));
    }

    let source_line = lines
        .next()
        .ok_or_else(|| PatchError::InvalidFormat("missing source path".into()))?;
    let source = source_line.strip_prefix("--- ").map(String::from);

    let target_line = lines
        .next()
        .ok_or_else(|| PatchError::InvalidFormat("missing target path".into()))?;
    let target = target_line.strip_prefix("+++ ").map(String::from);

    let mut ops = Vec::new();
    for line in lines {
        if line.is_empty() || line.starts_with("= ") {
            continue;
        }
        ops.push(parse_op_line(line)?);
    }

    Ok(BtrPatch { source, target, ops })
}

fn parse_op_line(line: &str) -> Result<PatchOp, PatchError> {
    if let Some(rest) = line.strip_prefix("~ ") {
        let (line_num, content) = split_at_pipe(rest)?;
        let segments = parse_segments(content);
        Ok(PatchOp::Modify {
            line: line_num,
            segments,
        })
    } else if let Some(rest) = line.strip_prefix("+ ") {
        let (pos, content) = split_at_pipe(rest)?;
        Ok(PatchOp::Insert {
            pos,
            content: content.to_string(),
        })
    } else if let Some(rest) = line.strip_prefix("- ") {
        parse_delete(rest)
    } else if let Some(rest) = line.strip_prefix("> ") {
        parse_move(rest)
    } else {
        Err(PatchError::InvalidFormat(format!(
            "unknown op prefix: '{line}'"
        )))
    }
}

fn split_at_pipe(s: &str) -> Result<(u32, &str), PatchError> {
    let (num_part, content) = s.split_once(" | ").ok_or_else(|| {
        PatchError::InvalidFormat(format!("expected '<num> | <content>', got '{s}'"))
    })?;
    let num: u32 = num_part
        .trim()
        .parse()
        .map_err(|e| PatchError::InvalidLineNumber(format!("{e}: '{num_part}'")))?;
    Ok((num, content))
}

fn parse_delete(s: &str) -> Result<PatchOp, PatchError> {
    let s = s.trim();
    if let Some((start_s, end_s)) = s.split_once(':') {
        let start: u32 = start_s
            .parse()
            .map_err(|e| PatchError::InvalidLineNumber(format!("{e}")))?;
        let end: u32 = end_s
            .parse()
            .map_err(|e| PatchError::InvalidLineNumber(format!("{e}")))?;
        Ok(PatchOp::Delete { start, end })
    } else {
        let line: u32 = s
            .parse()
            .map_err(|e| PatchError::InvalidLineNumber(format!("{e}")))?;
        Ok(PatchOp::Delete {
            start: line,
            end: line,
        })
    }
}

fn parse_move(s: &str) -> Result<PatchOp, PatchError> {
    let arrow = " \u{2192} ";
    let (range_part, to_part) = s.split_once(arrow).ok_or_else(|| {
        PatchError::InvalidFormat(format!("expected '<range> \u{2192} <pos>', got '{s}'"))
    })?;
    let range = range_part.trim();
    let to_after: u32 = to_part
        .trim()
        .parse()
        .map_err(|e| PatchError::InvalidLineNumber(format!("{e}")))?;

    let (start_s, end_s) = range.split_once(':').ok_or_else(|| {
        PatchError::InvalidFormat(format!("expected 'start:end', got '{range}'"))
    })?;
    let from_start: u32 = start_s
        .parse()
        .map_err(|e| PatchError::InvalidLineNumber(format!("{e}")))?;
    let from_end: u32 = end_s
        .parse()
        .map_err(|e| PatchError::InvalidLineNumber(format!("{e}")))?;

    Ok(PatchOp::Move {
        from_start,
        from_end,
        to_after,
    })
}

fn parse_segments(content: &str) -> Vec<PatchSegment> {
    let mut segments = Vec::new();
    let mut current_equal = String::new();
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '\\' && i + 1 < len {
            current_equal.push(chars[i + 1]);
            i += 2;
        } else if chars[i] == '{' {
            if !current_equal.is_empty() {
                segments.push(PatchSegment::Equal(std::mem::take(&mut current_equal)));
            }
            i += 1;
            let mut raw = String::new();
            while i < len {
                if chars[i] == '\\' && i + 1 < len {
                    raw.push(chars[i]);
                    raw.push(chars[i + 1]);
                    i += 2;
                } else if chars[i] == '}' {
                    i += 1;
                    break;
                } else {
                    raw.push(chars[i]);
                    i += 1;
                }
            }
            let (old, new) = split_change_raw(&raw);
            segments.push(PatchSegment::Change { old, new });
        } else {
            current_equal.push(chars[i]);
            i += 1;
        }
    }

    if !current_equal.is_empty() {
        segments.push(PatchSegment::Equal(current_equal));
    }

    segments
}

fn split_change_raw(raw: &str) -> (String, String) {
    let arrow = " \u{2192} ";
    let chars: Vec<char> = raw.chars().collect();
    let arrow_chars: Vec<char> = arrow.chars().collect();
    let alen = arrow_chars.len();

    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            i += 2;
        } else if i + alen <= chars.len() {
            let matches = chars[i..i + alen]
                .iter()
                .zip(arrow_chars.iter())
                .all(|(a, b)| a == b);
            if matches {
                let old_raw: String = chars[..i].iter().collect();
                let new_raw: String = chars[i + alen..].iter().collect();
                return (unescape_btr(&old_raw), unescape_btr(&new_raw));
            }
            i += 1;
        } else {
            i += 1;
        }
    }

    (unescape_btr(raw), String::new())
}

fn unescape_btr(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            result.push(chars[i + 1]);
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

// --- Applying ---

pub fn apply(ops: &[PatchOp], original: &str) -> Result<String, PatchError> {
    let orig_lines: Vec<&str> = original.lines().collect();
    let n = orig_lines.len();

    let mut deleted: HashSet<u32> = HashSet::new();
    let mut modified: HashMap<u32, &[PatchSegment]> = HashMap::new();
    let mut moved_from: HashSet<u32> = HashSet::new();
    let mut inserts_after: Vec<(u32, Vec<String>)> = Vec::new();
    let mut insert_map: HashMap<u32, Vec<usize>> = HashMap::new();

    for op in ops {
        match op {
            PatchOp::Delete { start, end } => {
                for l in *start..=*end {
                    deleted.insert(l);
                }
            }
            PatchOp::Modify { line, segments } => {
                modified.insert(*line, segments);
            }
            PatchOp::Insert { pos, content } => {
                let idx = inserts_after.len();
                insert_map.entry(*pos).or_default().push(idx);
                inserts_after.push((*pos, vec![content.clone()]));
            }
            PatchOp::Move {
                from_start,
                from_end,
                to_after,
            } => {
                for l in *from_start..=*from_end {
                    moved_from.insert(l);
                }
                let moved_lines: Vec<String> = (*from_start..=*from_end)
                    .filter_map(|l| {
                        orig_lines
                            .get((l as usize).wrapping_sub(1))
                            .map(|s| s.to_string())
                    })
                    .collect();
                let idx = inserts_after.len();
                insert_map.entry(*to_after).or_default().push(idx);
                inserts_after.push((*to_after, moved_lines));
            }
        }
    }

    let mut result: Vec<String> = Vec::new();

    if let Some(indices) = insert_map.get(&0) {
        for &idx in indices {
            result.extend(inserts_after[idx].1.iter().cloned());
        }
    }

    for i in 1..=(n as u32) {
        if !deleted.contains(&i) && !moved_from.contains(&i) {
            if let Some(segments) = modified.get(&i) {
                result.push(apply_segments(segments));
            } else {
                result.push(orig_lines[(i as usize) - 1].to_string());
            }
        }

        if let Some(indices) = insert_map.get(&i) {
            for &idx in indices {
                result.extend(inserts_after[idx].1.iter().cloned());
            }
        }
    }

    let has_trailing_newline = original.ends_with('\n') && !original.is_empty();
    let mut output = result.join("\n");
    if has_trailing_newline && !output.is_empty() {
        output.push('\n');
    }

    Ok(output)
}

fn apply_segments(segments: &[PatchSegment]) -> String {
    let mut line = String::new();
    for seg in segments {
        match seg {
            PatchSegment::Equal(s) => line.push_str(s),
            PatchSegment::Change { new, .. } => line.push_str(new),
        }
    }
    line
}
