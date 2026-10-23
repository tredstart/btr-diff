mod diff;
mod format;
mod token;

use clap::{Arg, Command};

fn main() {
    let matches = Command::new("btr-diff")
        .arg(Arg::new("f1").required(true))
        .arg(Arg::new("f2").required(true))
        .arg(Arg::new("visual").long("visual").action(clap::ArgAction::SetTrue))
        .get_matches();
    let f1 = matches.get_one::<String>("f1").expect("Missing argument");
    let f2 = matches.get_one::<String>("f2").expect("Missing argument");
    let visual = matches.get_flag("visual");

    std::thread::scope(|s| {
        let handles = [f1, f2].map(|f| {
            s.spawn(move || {
                let source = std::fs::read_to_string(f).expect("Cannot read the file!");
                token::tokenize(&source)
            })
        });
        let res: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().expect("Thread failed?"))
            .collect();
        let [a, b] = &res[0..2] else {
            panic!("Collected too many diffs for some reason, skipping for now...");
        };

        let ops = diff::diff(a, b);
        if visual {
            format::format_visual(&ops);
        } else {
            format::format_tsv(&ops);
        }
    });
}
