use crate::diff::{LineDiffOp, PreparedLine, Segment};

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

// --- btr/1 compact format (for agents) ---

pub fn format_btr(ops: &[LineDiffOp], path_a: &str, path_b: &str) {
    println!("btr/1");
    println!("--- {path_a}");
    println!("+++ {path_b}");

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
        flush_range(&mut del_start, del_end, "-");

        match op {
            LineDiffOp::Equal(_) => {}
            LineDiffOp::Insert { pos, content } => println!("+ {pos} | {content}"),
            LineDiffOp::Modify { line, segments } => {
                print!("~ {line} | ");
                render_segments_btr(segments);
                println!();
            }
            LineDiffOp::Move {
                from_start,
                from_end,
                to_after,
            } => println!("> {from_start}:{from_end} → {to_after}"),
            LineDiffOp::Delete(_) => unreachable!(),
        }
    }
    flush_range(&mut del_start, del_end, "-");
}

// --- btr/1 context format (for humans reviewing without the file) ---

pub fn format_btr_context(ops: &[LineDiffOp], path_a: &str, path_b: &str) {
    println!("btr/1");
    println!("--- {path_a}");
    println!("+++ {path_b}");

    let mut eq_start: Option<u32> = None;
    let mut eq_end: u32 = 0;
    let mut del_start: Option<u32> = None;
    let mut del_end: u32 = 0;

    for op in ops {
        match op {
            LineDiffOp::Equal(line) => {
                flush_range(&mut del_start, del_end, "-");
                if eq_start.is_none() {
                    eq_start = Some(*line);
                }
                eq_end = *line;
            }
            LineDiffOp::Delete(line) => {
                flush_range(&mut eq_start, eq_end, "=");
                if del_start.is_none() {
                    del_start = Some(*line);
                }
                del_end = *line;
            }
            _ => {
                flush_range(&mut eq_start, eq_end, "=");
                flush_range(&mut del_start, del_end, "-");
                match op {
                    LineDiffOp::Insert { pos, content } => println!("+ {pos} | {content}"),
                    LineDiffOp::Modify { line, segments } => {
                        print!("~ {line} | ");
                        render_segments_btr(segments);
                        println!();
                    }
                    LineDiffOp::Move {
                        from_start,
                        from_end,
                        to_after,
                    } => println!("> {from_start}:{from_end} → {to_after}"),
                    _ => {}
                }
            }
        }
    }
    flush_range(&mut eq_start, eq_end, "=");
    flush_range(&mut del_start, del_end, "-");
}

fn flush_range(start: &mut Option<u32>, end: u32, prefix: &str) {
    if let Some(s) = start.take() {
        if s == end {
            println!("{prefix} {s}");
        } else {
            println!("{prefix} {s}:{end}");
        }
    }
}

fn render_segments_btr(segments: &[Segment]) {
    for seg in segments {
        match seg {
            Segment::Equal(s) => print!("{s}"),
            Segment::Change { old, new } => print!("{{{old} → {new}}}"),
        }
    }
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
