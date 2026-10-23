# btr-diff

A structural diff tool that produces compact, token-efficient output designed for AI agents and human review.

## What it does

`btr-diff` compares two files and reports changes using the `btr/1` format:

- **Word-level precision** within modified lines -- shows exactly what changed, not the whole line
- **Move detection** -- recognizes code blocks that moved rather than reporting delete + insert
- **Range-collapsed deletions** -- consecutive deleted lines become a single range (`- 5:10`)
- **No noise** -- unchanged lines are omitted in the default format

## Output formats

### `btr` (default) -- compact, for agents

```
btr/1
--- old.rs
+++ new.rs
~ 2 |     let x = {42 -> 100};
+ 4 |     let z = transform(y);
- 9:10
> 12:15 -> 3
```

| Prefix | Meaning |
|--------|---------|
| `~`    | Modified line with inline `{old -> new}` segments |
| `+`    | Inserted line after the given position |
| `-`    | Deleted line or range |
| `>`    | Moved block: `from_start:from_end -> to_after` |

### `btr-context` -- includes unchanged line ranges for review without the file

### `visual` -- colored terminal output with inline red/green highlighting

## Usage

```
btr-diff <file1> <file2> [--format btr|btr-context|visual]
```

## Install

```
cargo install --path .
```

## License

This project is licensed under the [Mozilla Public License 2.0](LICENSE).
