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

// --- Visual format (adaptive hunk-based renderer) ---

const DIM: &str = "\x1b[2m";
const YELLOW: &str = "\x1b[33m";
const CONTEXT_LINES: usize = 2;

enum VisualLine<'a> {
    Context { line_a: u32, line_b: u32 },
    Delete { line_a: u32 },
    Insert { line_b: u32, content: &'a str },
    Modify { line_a: u32, line_b: u32, segments: &'a [Segment] },
    Move { from_start: u32, from_end: u32, to_after: u32 },
}

fn annotate_ops<'a>(ops: &'a [LineDiffOp]) -> Vec<VisualLine<'a>> {
    let mut result = Vec::new();
    let mut next_line_b: u32 = 1;
    for op in ops {
        match op {
            LineDiffOp::Equal(line_a) => {
                result.push(VisualLine::Context {
                    line_a: *line_a,
                    line_b: next_line_b,
                });
                next_line_b += 1;
            }
            LineDiffOp::Delete(line_a) => {
                result.push(VisualLine::Delete { line_a: *line_a });
            }
            LineDiffOp::Insert { content, line_b, .. } => {
                result.push(VisualLine::Insert {
                    line_b: *line_b,
                    content,
                });
                next_line_b = *line_b + 1;
            }
            LineDiffOp::Modify { line, segments } => {
                result.push(VisualLine::Modify {
                    line_a: *line,
                    line_b: next_line_b,
                    segments,
                });
                next_line_b += 1;
            }
            LineDiffOp::Move {
                from_start,
                from_end,
                to_after,
            } => {
                result.push(VisualLine::Move {
                    from_start: *from_start,
                    from_end: *from_end,
                    to_after: *to_after,
                });
                next_line_b += from_end - from_start + 1;
            }
        }
    }
    result
}

fn find_change_regions(visual_lines: &[VisualLine]) -> Vec<(usize, usize)> {
    let change_indices: Vec<usize> = visual_lines
        .iter()
        .enumerate()
        .filter(|(_, v)| !matches!(v, VisualLine::Context { .. }))
        .map(|(i, _)| i)
        .collect();

    if change_indices.is_empty() {
        return Vec::new();
    }

    let last_idx = visual_lines.len() - 1;
    let mut regions = Vec::new();
    let mut start = change_indices[0].saturating_sub(CONTEXT_LINES);
    let mut end = (change_indices[0] + CONTEXT_LINES).min(last_idx);

    for &ci in &change_indices[1..] {
        let new_start = ci.saturating_sub(CONTEXT_LINES);
        let new_end = (ci + CONTEXT_LINES).min(last_idx);
        if new_start <= end + 1 {
            end = new_end;
        } else {
            regions.push((start, end));
            start = new_start;
            end = new_end;
        }
    }
    regions.push((start, end));
    regions
}

fn reconstruct_line(prep_a: &[PreparedLine], line_a: u32) -> String {
    prep_a[(line_a as usize) - 1]
        .tokens
        .iter()
        .map(|t| t.seq.as_str())
        .collect()
}

fn push_colored_segment(
    out: &mut String,
    text: &str,
    color: &str,
    visible_len: &mut usize,
    max_width: usize,
) {
    let remaining = max_width.saturating_sub(*visible_len);
    if remaining == 0 {
        return;
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= remaining {
        if !color.is_empty() {
            out.push_str(color);
        }
        out.push_str(text);
        if !color.is_empty() {
            out.push_str(RESET);
        }
        *visible_len += chars.len();
    } else {
        let take = if remaining > 2 { remaining - 2 } else { remaining };
        let truncated: String = chars[..take].iter().collect();
        if !color.is_empty() {
            out.push_str(color);
        }
        out.push_str(&truncated);
        if !color.is_empty() {
            out.push_str(RESET);
        }
        if remaining > 2 {
            out.push_str("..");
        }
        *visible_len = max_width;
    }
}

fn pad_to_width(out: &mut String, visible_len: &mut usize, target: usize) {
    while *visible_len < target {
        out.push(' ');
        *visible_len += 1;
    }
}

fn truncate_plain(text: &str, max_width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_width {
        text.to_string()
    } else if max_width > 2 {
        let mut result: String = chars[..max_width - 2].iter().collect();
        result.push_str("..");
        result
    } else {
        chars[..max_width].iter().collect()
    }
}

fn pad_plain(text: &str, target_width: usize) -> String {
    let char_len = text.chars().count();
    if char_len >= target_width {
        return text.to_string();
    }
    let mut result = text.to_string();
    for _ in 0..target_width - char_len {
        result.push(' ');
    }
    result
}

fn render_separator(out: &mut String, count: usize) {
    writeln!(out, "{DIM}       ... {count} lines unchanged ...{RESET}").unwrap();
}

fn write_side_by_side_row(
    out: &mut String,
    left_num: &str,
    left_cell: &str,
    right_num: &str,
    right_cell: &str,
) {
    writeln!(out, "{left_num} | {left_cell} | {right_num} | {right_cell}").unwrap();
}

fn flush_pending_deletes_inserts(
    out: &mut String,
    visual_lines: &[VisualLine],
    prep_a: &[PreparedLine],
    pending_deletes: &mut Vec<usize>,
    pending_inserts: &mut Vec<usize>,
    col_width: usize,
) {
    let blank_num = "    ";
    let blank_cell = &" ".repeat(col_width);
    let pairs = pending_deletes.len().min(pending_inserts.len());

    for k in 0..pairs {
        if let VisualLine::Delete { line_a } = &visual_lines[pending_deletes[k]] {
            let text = reconstruct_line(prep_a, *line_a);
            let mut left_cell = String::new();
            let mut left_vis = 0;
            push_colored_segment(&mut left_cell, &text, RED, &mut left_vis, col_width);
            pad_to_width(&mut left_cell, &mut left_vis, col_width);

            if let VisualLine::Insert { line_b, content } = &visual_lines[pending_inserts[k]] {
                let mut right_cell = String::new();
                let mut right_vis = 0;
                push_colored_segment(&mut right_cell, content, GREEN, &mut right_vis, col_width);
                write_side_by_side_row(
                    out,
                    &format!("{line_a:>4}"),
                    &left_cell,
                    &format!("{line_b:>4}"),
                    &right_cell,
                );
            }
        }
    }

    for &di in &pending_deletes[pairs..] {
        if let VisualLine::Delete { line_a } = &visual_lines[di] {
            let text = reconstruct_line(prep_a, *line_a);
            let mut left_cell = String::new();
            let mut left_vis = 0;
            push_colored_segment(&mut left_cell, &text, RED, &mut left_vis, col_width);
            pad_to_width(&mut left_cell, &mut left_vis, col_width);
            write_side_by_side_row(out, &format!("{line_a:>4}"), &left_cell, blank_num, "");
        }
    }

    for &ii in &pending_inserts[pairs..] {
        if let VisualLine::Insert { line_b, content } = &visual_lines[ii] {
            let mut right_cell = String::new();
            let mut right_vis = 0;
            push_colored_segment(&mut right_cell, content, GREEN, &mut right_vis, col_width);
            write_side_by_side_row(
                out,
                blank_num,
                blank_cell,
                &format!("{line_b:>4}"),
                &right_cell,
            );
        }
    }

    pending_deletes.clear();
    pending_inserts.clear();
}

fn render_side_by_side_region(
    out: &mut String,
    visual_lines: &[VisualLine],
    prep_a: &[PreparedLine],
    start: usize,
    end: usize,
    col_width: usize,
) {
    let mut pending_deletes: Vec<usize> = Vec::new();
    let mut pending_inserts: Vec<usize> = Vec::new();

    for i in start..=end {
        match &visual_lines[i] {
            VisualLine::Delete { .. } => {
                pending_deletes.push(i);
                continue;
            }
            VisualLine::Insert { .. } => {
                pending_inserts.push(i);
                continue;
            }
            _ => {}
        }

        flush_pending_deletes_inserts(
            out,
            visual_lines,
            prep_a,
            &mut pending_deletes,
            &mut pending_inserts,
            col_width,
        );

        match &visual_lines[i] {
            VisualLine::Context { line_a, line_b } => {
                let text = reconstruct_line(prep_a, *line_a);
                let left_text = pad_plain(&truncate_plain(&text, col_width), col_width);
                let right_text = truncate_plain(&text, col_width);
                write_side_by_side_row(
                    out,
                    &format!("{DIM}{line_a:>4}{RESET}"),
                    &format!("{DIM}{left_text}{RESET}"),
                    &format!("{DIM}{line_b:>4}{RESET}"),
                    &format!("{DIM}{right_text}{RESET}"),
                );
            }
            VisualLine::Modify {
                line_a,
                line_b,
                segments,
            } => {
                let mut left_cell = String::new();
                let mut left_vis = 0;
                for seg in *segments {
                    if left_vis >= col_width {
                        break;
                    }
                    match seg {
                        Segment::Equal(t) => {
                            push_colored_segment(&mut left_cell, t, "", &mut left_vis, col_width)
                        }
                        Segment::Change { old, .. } => {
                            push_colored_segment(&mut left_cell, old, RED, &mut left_vis, col_width)
                        }
                    }
                }
                pad_to_width(&mut left_cell, &mut left_vis, col_width);

                let mut right_cell = String::new();
                let mut right_vis = 0;
                for seg in *segments {
                    if right_vis >= col_width {
                        break;
                    }
                    match seg {
                        Segment::Equal(t) => {
                            push_colored_segment(
                                &mut right_cell,
                                t,
                                "",
                                &mut right_vis,
                                col_width,
                            )
                        }
                        Segment::Change { new, .. } => {
                            push_colored_segment(
                                &mut right_cell,
                                new,
                                GREEN,
                                &mut right_vis,
                                col_width,
                            )
                        }
                    }
                }

                write_side_by_side_row(
                    out,
                    &format!("{line_a:>4}"),
                    &left_cell,
                    &format!("{line_b:>4}"),
                    &right_cell,
                );
            }
            VisualLine::Move {
                from_start,
                from_end,
                to_after,
            } => {
                writeln!(out,
                    "{YELLOW}     -- lines {from_start}-{from_end} moved to after line {to_after} --{RESET}"
                ).unwrap();
            }
            _ => {}
        }
    }

    flush_pending_deletes_inserts(
        out,
        visual_lines,
        prep_a,
        &mut pending_deletes,
        &mut pending_inserts,
        col_width,
    );
}

pub fn format_visual_string(
    ops: &[LineDiffOp],
    prep_a: &[PreparedLine],
    width: usize,
) -> String {
    let visual_lines = annotate_ops(ops);
    if visual_lines.is_empty() {
        return String::new();
    }

    let regions = find_change_regions(&visual_lines);
    if regions.is_empty() {
        let mut result = String::new();
        render_separator(&mut result, visual_lines.len());
        return result;
    }

    let col_width = (width.saturating_sub(17) / 2).max(10);
    let mut result = String::new();
    let mut position = 0;

    for &(region_start, region_end) in &regions {
        if region_start > position {
            render_separator(&mut result, region_start - position);
        }
        render_side_by_side_region(
            &mut result,
            &visual_lines,
            prep_a,
            region_start,
            region_end,
            col_width,
        );
        position = region_end + 1;
    }

    if position < visual_lines.len() {
        render_separator(&mut result, visual_lines.len() - position);
    }

    result
}

pub fn format_visual(ops: &[LineDiffOp], prep_a: &[PreparedLine], width: usize) {
    print!("{}", format_visual_string(ops, prep_a, width));
}
