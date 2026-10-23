mod diff;
mod format;
mod token;

use clap::{Arg, Command};

fn main() {
    let matches = Command::new("btr-diff")
        .arg(Arg::new("f1").required(true))
        .arg(Arg::new("f2").required(true))
        .arg(
            Arg::new("format")
                .long("format")
                .value_parser(["btr", "btr-context", "visual"])
                .default_value("btr"),
        )
        .arg(
            Arg::new("visual")
                .long("visual")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let f1 = matches.get_one::<String>("f1").unwrap();
    let f2 = matches.get_one::<String>("f2").unwrap();
    let visual = matches.get_flag("visual");
    let fmt = matches.get_one::<String>("format").unwrap().as_str();

    std::thread::scope(|s| {
        let handles = [f1, f2].map(|f| {
            s.spawn(move || {
                let source = std::fs::read_to_string(f).expect("Cannot read the file!");
                diff::prepare_lines(&source)
            })
        });
        let res: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().expect("Thread failed"))
            .collect();
        let [prep_a, prep_b] = &res[0..2] else {
            unreachable!()
        };

        let ops = diff::diff_lines(prep_a, prep_b);

        if visual || fmt == "visual" {
            format::format_visual(&ops, prep_a);
        } else if fmt == "btr-context" {
            format::format_btr_context(&ops, f1, f2);
        } else {
            format::format_btr(&ops, f1, f2);
        }
    });
}
