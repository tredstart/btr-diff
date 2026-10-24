use crate::diff::{LineDiffOp, PreparedLine, Segment};
use std::fmt::Write;

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

// --- Escaping for round-trip-safe btr/1 output ---

pub fn escape_btr_equal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            _ => out.push(c),
        }
    }
    out
}

pub fn escape_btr_change(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '}' => out.push_str("\\}"),
            '\u{2192}' => out.push_str("\\\u{2192}"),
            _ => out.push(c),
        }
    }
    out
}

fn render_segments_btr_escaped(segments: &[Segment]) -> String {
    let mut s = String::new();
    for seg in segments {
        match seg {
            Segment::Equal(text) => s.push_str(&escape_btr_equal(text)),
            Segment::Change { old, new } => {
                s.push('{');
                s.push_str(&escape_btr_change(old));
                s.push_str(" \u{2192} ");
                s.push_str(&escape_btr_change(new));
                s.push('}');
            }
        }
    }
    s
}

// --- btr/1 compact format (for agents) ---

pub fn format_btr_string(ops: &[LineDiffOp], path_a: &str, path_b: &str) -> String {
    let mut s = String::new();
    writeln!(s, "btr/1").unwrap();
    writeln!(s, "--- {path_a}").unwrap();
    writeln!(s, "+++ {path_b}").unwrap();

    let mut del_start: Option<u32> = None;
    let mut del_end: u32 = 0;

    for op in ops {
        if let LineDiffOp::Delete(line) = op {
            if del_start.is_none() {
                del_start = Some(*line);
            }
            del_end = *line;
            continue;
        }
        flush_range_str(&mut s, &mut del_start, del_end, "-");

        match op {
            LineDiffOp::Equal(_) => {}
            LineDiffOp::Insert { pos, content, .. } => {
                writeln!(s, "+ {pos} | {content}").unwrap();
            }
            LineDiffOp::Modify { line, segments } => {
                write!(s, "~ {line} | ").unwrap();
                s.push_str(&render_segments_btr_escaped(segments));
                writeln!(s).unwrap();
            }
            LineDiffOp::Move {
                from_start,
                from_end,
                to_after,
            } => {
                writeln!(s, "> {from_start}:{from_end} \u{2192} {to_after}").unwrap();
            }
            LineDiffOp::Delete(_) => unreachable!(),
        }
    }
    flush_range_str(&mut s, &mut del_start, del_end, "-");
    s
}

pub fn format_btr(ops: &[LineDiffOp], path_a: &str, path_b: &str) {
    print!("{}", format_btr_string(ops, path_a, path_b));
}

fn flush_range_str(s: &mut String, start: &mut Option<u32>, end: u32, prefix: &str) {
    if let Some(st) = start.take() {
        if st == end {
            writeln!(s, "{prefix} {st}").unwrap();
        } else {
            writeln!(s, "{prefix} {st}:{end}").unwrap();
        }
    }
}

// --- btr/1 context format (for humans reviewing without the file) ---

pub fn format_btr_context(ops: &[LineDiffOp], path_a: &str, path_b: &str) {
    print!("{}", format_btr_context_string(ops, path_a, path_b));
}

pub fn format_btr_context_string(ops: &[LineDiffOp], path_a: &str, path_b: &str) -> String {
    let mut s = String::new();
    writeln!(s, "btr/1").unwrap();
    writeln!(s, "--- {path_a}").unwrap();
    writeln!(s, "+++ {path_b}").unwrap();

    let mut eq_start: Option<u32> = None;
    let mut eq_end: u32 = 0;
    let mut del_start: Option<u32> = None;
    let mut del_end: u32 = 0;

    for op in ops {
        match op {
            LineDiffOp::Equal(line) => {
                flush_range_str(&mut s, &mut del_start, del_end, "-");
                if eq_start.is_none() {
                    eq_start = Some(*line);
                }
                eq_end = *line;
            }
            LineDiffOp::Delete(line) => {
                flush_range_str(&mut s, &mut eq_start, eq_end, "=");
                if del_start.is_none() {
                    del_start = Some(*line);
                }
                del_end = *line;
            }
            _ => {
                flush_range_str(&mut s, &mut eq_start, eq_end, "=");
                flush_range_str(&mut s, &mut del_start, del_end, "-");
                match op {
                    LineDiffOp::Insert { pos, content, .. } => {
                        writeln!(s, "+ {pos} | {content}").unwrap();
                    }
                    LineDiffOp::Modify { line, segments } => {
                        write!(s, "~ {line} | ").unwrap();
                        s.push_str(&render_segments_btr_escaped(segments));
                        writeln!(s).unwrap();
                    }
                    LineDiffOp::Move {
                        from_start,
                        from_end,
                        to_after,
                    } => {
                        writeln!(s, "> {from_start}:{from_end} \u{2192} {to_after}").unwrap();
                    }
                    _ => {}
                }
            }
        }
    }
    flush_range_str(&mut s, &mut eq_start, eq_end, "=");
    flush_range_str(&mut s, &mut del_start, del_end, "-");
    s
}

// --- Visual format (colored inline, using line-level ops) ---

pub fn format_visual(ops: &[LineDiffOp], prep_a: &[PreparedLine]) {
    for op in ops {
        match op {
            LineDiffOp::Equal(line) => {
                let text: String = prep_a[(*line as usize) - 1]
                    .tokens
                    .iter()
                    .map(|t| t.seq.as_str())
                    .collect();
                println!("{text}");
            }
            LineDiffOp::Delete(line) => {
                let text: String = prep_a[(*line as usize) - 1]
                    .tokens
                    .iter()
                    .map(|t| t.seq.as_str())
                    .collect();
                println!("{RED}{text}{RESET}");
            }
            LineDiffOp::Insert { content, .. } => {
                println!("{GREEN}{content}{RESET}");
            }
            LineDiffOp::Modify { segments, .. } => {
                for seg in segments {
                    match seg {
                        Segment::Equal(s) => print!("{s}"),
                        Segment::Change { old, new } => {
                            print!("{RED}{old}{RESET}{GREEN}{new}{RESET}");
                        }
                    }
                }
                println!();
            }
            LineDiffOp::Move {
                from_start,
                from_end,
                to_after,
            } => {
                println!("{RED}> moved {from_start}:{from_end} → after {to_after}{RESET}");
            }
        }
    }
}
