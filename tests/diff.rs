use btr_diff::diff::{self, LineDiffOp, Segment};

fn seg_texts(segments: &[Segment]) -> Vec<String> {
    segments
        .iter()
        .map(|s| match s {
            Segment::Equal(t) => format!("={t}"),
            Segment::Change { old, new } => format!("~{old}>{new}"),
        })
        .collect()
}

// --- word_diff ---

#[test]
fn word_diff_identical() {
    let src = "let x = 10;";
    let prep = diff::prepare_lines(src);
    let segs = diff::word_diff(&prep[0].tokens, &prep[0].tokens);
    assert!(segs.iter().all(|s| matches!(s, Segment::Equal(_))));
}

#[test]
fn word_diff_single_replace() {
    let a = diff::prepare_lines("let x = 10;");
    let b = diff::prepare_lines("let x = 20;");
    let segs = diff::word_diff(&a[0].tokens, &b[0].tokens);
    let texts = seg_texts(&segs);
    assert!(texts.iter().any(|t| t.contains("~") && t.contains("10") && t.contains("20")));
}

#[test]
fn word_diff_multiple_changes() {
    let a = diff::prepare_lines("fn sum(a: i32) -> i32");
    let b = diff::prepare_lines("fn product(a: i64) -> i64");
    let segs = diff::word_diff(&a[0].tokens, &b[0].tokens);
    let texts = seg_texts(&segs);
    let changes: Vec<_> = texts.iter().filter(|t| t.starts_with('~')).collect();
    assert!(changes.len() >= 2);
}

// --- diff_lines ---

#[test]
fn identical_files() {
    let src = "line one\nline two\nline three";
    let prep_a = diff::prepare_lines(src);
    let prep_b = diff::prepare_lines(src);
    let ops = diff::diff_lines(&prep_a, &prep_b);
    assert!(ops.iter().all(|op| matches!(op, LineDiffOp::Equal(_))));
    assert_eq!(ops.len(), 3);
}

#[test]
fn pure_insert() {
    let a = "line one\nline three";
    let b = "line one\nline two\nline three";
    let prep_a = diff::prepare_lines(a);
    let prep_b = diff::prepare_lines(b);
    let ops = diff::diff_lines(&prep_a, &prep_b);

    let inserts: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, LineDiffOp::Insert { .. }))
        .collect();
    assert_eq!(inserts.len(), 1);
    if let LineDiffOp::Insert { content, .. } = inserts[0] {
        assert_eq!(content, "line two");
    }
}

#[test]
fn pure_delete() {
    let a = "line one\nline two\nline three";
    let b = "line one\nline three";
    let prep_a = diff::prepare_lines(a);
    let prep_b = diff::prepare_lines(b);
    let ops = diff::diff_lines(&prep_a, &prep_b);

    let deletes: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, LineDiffOp::Delete(_)))
        .collect();
    assert_eq!(deletes.len(), 1);
    if let LineDiffOp::Delete(line) = deletes[0] {
        assert_eq!(*line, 2);
    }
}

#[test]
fn modify_produces_segments() {
    let a = "let x = 10;";
    let b = "let x = 20;";
    let prep_a = diff::prepare_lines(a);
    let prep_b = diff::prepare_lines(b);
    let ops = diff::diff_lines(&prep_a, &prep_b);

    assert_eq!(ops.len(), 1);
    if let LineDiffOp::Modify { line, segments } = &ops[0] {
        assert_eq!(*line, 1);
        let has_change = segments
            .iter()
            .any(|s| matches!(s, Segment::Change { .. }));
        assert!(has_change);
    } else {
        panic!("expected Modify, got {:?}", ops[0]);
    }
}

#[test]
fn multi_line_mixed() {
    let a = "keep\nold value\nkeep too";
    let b = "keep\nnew value\nkeep too\nadded";
    let prep_a = diff::prepare_lines(a);
    let prep_b = diff::prepare_lines(b);
    let ops = diff::diff_lines(&prep_a, &prep_b);

    let equals = ops.iter().filter(|op| matches!(op, LineDiffOp::Equal(_))).count();
    let modifies = ops.iter().filter(|op| matches!(op, LineDiffOp::Modify { .. })).count();
    let inserts = ops.iter().filter(|op| matches!(op, LineDiffOp::Insert { .. })).count();

    assert_eq!(equals, 2);
    assert_eq!(modifies, 1);
    assert_eq!(inserts, 1);
}

#[test]
fn empty_files() {
    let prep_a = diff::prepare_lines("");
    let prep_b = diff::prepare_lines("");
    let ops = diff::diff_lines(&prep_a, &prep_b);
    assert!(ops.is_empty());
}

// --- move detection ---

#[test]
fn detects_moved_block() {
    let a = "HEADER\nFOOTER\nmove_1\nmove_2\nmove_3";
    let b = "move_1\nmove_2\nmove_3\nHEADER\nFOOTER";
    let prep_a = diff::prepare_lines(a);
    let prep_b = diff::prepare_lines(b);
    let ops = diff::diff_lines(&prep_a, &prep_b);

    let moves: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, LineDiffOp::Move { .. }))
        .collect();
    assert!(!moves.is_empty(), "expected a Move op, got: {:?}", ops);
    if let LineDiffOp::Move { from_start, from_end, .. } = moves[0] {
        assert_eq!(*from_start, 1);
        assert_eq!(*from_end, 2);
    }
}

#[test]
fn no_false_moves() {
    let a = "aaa\nbbb";
    let b = "ccc\nddd";
    let prep_a = diff::prepare_lines(a);
    let prep_b = diff::prepare_lines(b);
    let ops = diff::diff_lines(&prep_a, &prep_b);

    let moves = ops.iter().filter(|op| matches!(op, LineDiffOp::Move { .. })).count();
    assert_eq!(moves, 0);
}

// --- prepare_lines caching ---

#[test]
fn same_line_same_hash() {
    let prep = diff::prepare_lines("aaa\naaa\nbbb");
    assert_eq!(prep[0].hash, prep[1].hash);
    assert_ne!(prep[0].hash, prep[2].hash);
}
