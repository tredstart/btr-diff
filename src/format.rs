use crate::diff::DiffOp;

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

pub fn format_visual(ops: &[DiffOp]) {
    for op in ops {
        match op {
            DiffOp::Equal(t) => print!("{}", t.seq),
            DiffOp::Insert(t) => print!("{GREEN}{}{RESET}", t.seq),
            DiffOp::Delete(t) => print!("{RED}{}{RESET}", t.seq),
            DiffOp::Replace(old, new) => {
                print!("{RED}{}{RESET}{GREEN}{}{RESET}", old.seq, new.seq);
            }
        }
    }
    println!();
}

pub fn format_tsv(ops: &[DiffOp]) {
    for op in ops {
        match op {
            DiffOp::Equal(t) => println!("=\t{}", t.seq),
            DiffOp::Insert(t) => println!("+\t{}", t.seq),
            DiffOp::Delete(t) => println!("-\t{}", t.seq),
            DiffOp::Replace(old, new) => println!("~\t{}\t{}", old.seq, new.seq),
        }
    }
}
