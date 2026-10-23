use btr_diff::token::tokenize;

#[test]
fn simple_words() {
    let tokens = tokenize("hello world");
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].seq, "hello");
    assert_eq!(tokens[1].seq, " ");
    assert_eq!(tokens[2].seq, "world");
}

#[test]
fn preserves_multiple_spaces() {
    let tokens = tokenize("a  b");
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].seq, "a");
    assert_eq!(tokens[1].seq, "  ");
    assert_eq!(tokens[2].seq, "b");
}

#[test]
fn line_tracking() {
    let tokens = tokenize("first\nsecond\nthird");
    let words: Vec<_> = tokens.iter().filter(|t| !t.seq.trim().is_empty()).collect();
    assert_eq!(words[0].seq, "first");
    assert_eq!(words[0].line, 1);
    assert_eq!(words[1].seq, "second");
    assert_eq!(words[1].line, 2);
    assert_eq!(words[2].seq, "third");
    assert_eq!(words[2].line, 3);
}

#[test]
fn equal_strings_equal_hashes() {
    let a = tokenize("foo");
    let b = tokenize("foo");
    assert_eq!(a[0].hash, b[0].hash);
}

#[test]
fn different_strings_different_hashes() {
    let a = tokenize("foo");
    let b = tokenize("bar");
    assert_ne!(a[0].hash, b[0].hash);
}

#[test]
fn empty_input() {
    let tokens = tokenize("");
    assert_eq!(tokens.len(), 1);
    assert!(tokens[0].is_empty());
}
