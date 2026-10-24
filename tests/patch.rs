use btr_diff::diff;
use btr_diff::format;
use btr_diff::patch;

fn round_trip(a: &str, b: &str) {
    // Forward: a → b
    let prep_a = diff::prepare_lines(a);
    let prep_b = diff::prepare_lines(b);
    let ops = diff::diff_lines(&prep_a, &prep_b);
    let patch_text = format::format_btr_string(&ops, "a", "b");
    let parsed = patch::parse(&patch_text).unwrap_or_else(|e| {
        panic!("parse forward failed: {e}\npatch:\n{patch_text}");
    });
    let result = patch::apply(&parsed.ops, a).unwrap_or_else(|e| {
        panic!("apply forward failed: {e}\npatch:\n{patch_text}");
    });
    assert_eq!(result, b, "forward a→b failed\npatch:\n{patch_text}");

    // Reverse: b → a
    let ops_rev = diff::diff_lines(&prep_b, &prep_a);
    let patch_rev_text = format::format_btr_string(&ops_rev, "b", "a");
    let parsed_rev = patch::parse(&patch_rev_text).unwrap_or_else(|e| {
        panic!("parse reverse failed: {e}\npatch:\n{patch_rev_text}");
    });
    let result_rev = patch::apply(&parsed_rev.ops, b).unwrap_or_else(|e| {
        panic!("apply reverse failed: {e}\npatch:\n{patch_rev_text}");
    });
    assert_eq!(
        result_rev, a,
        "reverse b→a failed\npatch:\n{patch_rev_text}"
    );
}

// --- Round-trip tests ---

#[test]
fn rt_simple_modify() {
    round_trip("let x = 42;", "let x = 100;");
}

#[test]
fn rt_modify_multiple_words() {
    round_trip(
        "fn sum(a: i32) -> i32",
        "fn product(a: i64) -> i64",
    );
}

#[test]
fn rt_insert_middle() {
    round_trip("aaa\nccc", "aaa\nbbb\nccc");
}

#[test]
fn rt_insert_beginning() {
    round_trip("bbb\nccc", "aaa\nbbb\nccc");
}

#[test]
fn rt_insert_end() {
    round_trip("aaa\nbbb", "aaa\nbbb\nccc");
}

#[test]
fn rt_delete_single() {
    round_trip("aaa\nbbb\nccc", "aaa\nccc");
}

#[test]
fn rt_delete_range() {
    round_trip("aaa\nbbb\nccc\nddd\neee", "aaa\neee");
}

#[test]
fn rt_delete_first() {
    round_trip("aaa\nbbb\nccc", "bbb\nccc");
}

#[test]
fn rt_delete_last() {
    round_trip("aaa\nbbb\nccc", "aaa\nbbb");
}

#[test]
fn rt_mixed_operations() {
    round_trip(
        "keep\nold value\nremove me\nkeep too",
        "keep\nnew value\nkeep too\nadded",
    );
}

#[test]
fn rt_all_lines_modified() {
    round_trip("alpha\nbeta\ngamma", "ALPHA\nBETA\nGAMMA");
}

#[test]
fn rt_braces_in_code() {
    round_trip(
        "fn f() { return 42; }",
        "fn g() { return 100; }",
    );
}

#[test]
fn rt_nested_braces() {
    round_trip(
        "if (x > 0) { vec.push(x); }",
        "if (y > 0) { vec.push(y); }",
    );
}

#[test]
fn rt_curly_brace_content() {
    round_trip(
        "let map = HashMap::new();",
        "let map = BTreeMap::new();",
    );
}

#[test]
fn rt_multiline_with_braces() {
    round_trip(
        "fn foo() {\n    let x = 42;\n    return x;\n}",
        "fn bar() {\n    let x = 100;\n    let y = x + 1;\n    return y;\n}",
    );
}

#[test]
fn rt_single_line_file() {
    round_trip("hello world", "goodbye world");
}

#[test]
fn rt_add_to_single_line() {
    round_trip("first", "first\nsecond\nthird");
}

#[test]
fn rt_reduce_to_single_line() {
    round_trip("first\nsecond\nthird", "first");
}

#[test]
fn rt_completely_different() {
    round_trip("aaa\nbbb\nccc", "xxx\nyyy\nzzz");
}

#[test]
fn rt_identical_files() {
    round_trip("no change\nhere", "no change\nhere");
}

#[test]
fn rt_whitespace_changes() {
    round_trip("  indented", "    more indented");
}

#[test]
fn rt_large_scattered_changes() {
    let a = (1..=20)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let b = (1..=20)
        .map(|i| {
            if i % 5 == 0 {
                format!("modified line {i}")
            } else if i % 7 == 0 {
                String::new()
            } else {
                format!("line {i}")
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    round_trip(&a, &b);
}

#[test]
fn rt_realistic_rust_refactor() {
    let a = "\
use std::collections::HashMap;

fn process(data: &[u8]) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for chunk in data.chunks(4) {
        let key = String::from_utf8_lossy(chunk).to_string();
        *map.entry(key).or_insert(0) += 1;
    }
    map
}";

    let b = "\
use std::collections::BTreeMap;

fn process(data: &[u8]) -> BTreeMap<String, usize> {
    let mut map = BTreeMap::new();
    for chunk in data.chunks(8) {
        let key = String::from_utf8_lossy(chunk).to_string();
        let count = map.entry(key).or_insert(0);
        *count += 1;
    }
    map
}";
    round_trip(a, b);
}

#[test]
fn rt_insert_multiple_adjacent() {
    round_trip("a\nd", "a\nb\nc\nd");
}

#[test]
fn rt_delete_all_but_one() {
    round_trip("a\nb\nc\nd\ne", "c");
}

// --- Parse-specific tests ---

#[test]
fn parse_minimal_patch() {
    let input = "btr/1\n--- a\n+++ b\n~ 1 | {hello → world}\n";
    let parsed = patch::parse(input).unwrap();
    assert_eq!(parsed.source, Some("a".into()));
    assert_eq!(parsed.target, Some("b".into()));
    assert_eq!(parsed.ops.len(), 1);
}

#[test]
fn parse_delete_range() {
    let input = "btr/1\n--- a\n+++ b\n- 3:7\n";
    let parsed = patch::parse(input).unwrap();
    assert_eq!(
        parsed.ops[0],
        patch::PatchOp::Delete { start: 3, end: 7 }
    );
}

#[test]
fn parse_delete_single() {
    let input = "btr/1\n--- a\n+++ b\n- 5\n";
    let parsed = patch::parse(input).unwrap();
    assert_eq!(
        parsed.ops[0],
        patch::PatchOp::Delete { start: 5, end: 5 }
    );
}

#[test]
fn parse_insert() {
    let input = "btr/1\n--- a\n+++ b\n+ 3 | new content here\n";
    let parsed = patch::parse(input).unwrap();
    assert_eq!(
        parsed.ops[0],
        patch::PatchOp::Insert {
            pos: 3,
            content: "new content here".into()
        }
    );
}

#[test]
fn parse_move() {
    let input = "btr/1\n--- a\n+++ b\n> 5:8 \u{2192} 2\n";
    let parsed = patch::parse(input).unwrap();
    assert_eq!(
        parsed.ops[0],
        patch::PatchOp::Move {
            from_start: 5,
            from_end: 8,
            to_after: 2
        }
    );
}

#[test]
fn parse_escaped_braces_in_equal() {
    let input = "btr/1\n--- a\n+++ b\n~ 1 | if \\{ {x → y} \\}\n";
    let parsed = patch::parse(input).unwrap();
    if let patch::PatchOp::Modify { segments, .. } = &parsed.ops[0] {
        assert_eq!(
            segments[0],
            patch::PatchSegment::Equal("if { ".into())
        );
        assert_eq!(
            segments[1],
            patch::PatchSegment::Change {
                old: "x".into(),
                new: "y".into()
            }
        );
        assert_eq!(
            segments[2],
            patch::PatchSegment::Equal(" }".into())
        );
    } else {
        panic!("expected Modify op");
    }
}

#[test]
fn parse_skips_context_equal_lines() {
    let input = "btr/1\n--- a\n+++ b\n= 1:3\n~ 4 | {old → new}\n= 5:8\n";
    let parsed = patch::parse(input).unwrap();
    assert_eq!(parsed.ops.len(), 1);
}

#[test]
fn parse_invalid_header() {
    let input = "wrong/1\n--- a\n+++ b\n";
    assert!(patch::parse(input).is_err());
}

#[test]
fn parse_empty_input() {
    assert!(patch::parse("").is_err());
}

// --- Apply-specific tests ---

#[test]
fn apply_no_ops() {
    let result = patch::apply(&[], "hello\nworld").unwrap();
    assert_eq!(result, "hello\nworld");
}

#[test]
fn apply_preserves_trailing_newline() {
    let ops = vec![patch::PatchOp::Modify {
        line: 1,
        segments: vec![patch::PatchSegment::Change {
            old: "hello".into(),
            new: "goodbye".into(),
        }],
    }];
    let result = patch::apply(&ops, "hello\nworld\n").unwrap();
    assert_eq!(result, "goodbye\nworld\n");
}

#[test]
fn apply_no_trailing_newline() {
    let ops = vec![patch::PatchOp::Modify {
        line: 1,
        segments: vec![patch::PatchSegment::Change {
            old: "hello".into(),
            new: "goodbye".into(),
        }],
    }];
    let result = patch::apply(&ops, "hello\nworld").unwrap();
    assert_eq!(result, "goodbye\nworld");
}
