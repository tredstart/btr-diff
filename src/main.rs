mod diff;
mod format;
mod patch;
mod token;

use clap::{Arg, Command};
use std::io::Read;

fn main() {
    let matches = Command::new("btr-diff")
        .subcommand_negates_reqs(true)
        .subcommand(
            Command::new("apply")
                .about("Apply a btr/1 patch to a file")
                .arg(Arg::new("patch").required(true).help("Path to the btr patch file"))
                .arg(
                    Arg::new("original")
                        .required(true)
                        .help("Path to the original file"),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .help("Output file (default: stdout)"),
                ),
        )
        .arg(Arg::new("f1").required_unless_present("stdin"))
        .arg(Arg::new("f2").required_unless_present("stdin"))
        .arg(
            Arg::new("stdin")
                .long("stdin")
                .action(clap::ArgAction::SetTrue)
                .help("Read two documents from stdin separated by a null byte"),
        )
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
        .arg(
            Arg::new("width")
                .long("width")
                .short('w')
                .value_parser(clap::value_parser!(usize))
                .default_value("80")
                .help("Terminal width for visual format"),
        )
        .get_matches();

    if let Some(apply_matches) = matches.subcommand_matches("apply") {
        run_apply(apply_matches);
    } else if matches.get_flag("stdin") {
        run_diff_stdin(&matches);
    } else {
        run_diff(&matches);
    }
}

fn run_apply(matches: &clap::ArgMatches) {
    let patch_path = matches.get_one::<String>("patch").unwrap();
    let original_path = matches.get_one::<String>("original").unwrap();
    let output_path = matches.get_one::<String>("output");

    let patch_text = std::fs::read_to_string(patch_path).expect("Cannot read patch file");
    let original = std::fs::read_to_string(original_path).expect("Cannot read original file");

    let parsed = patch::parse(&patch_text).unwrap_or_else(|e| {
        eprintln!("Error parsing patch: {e}");
        std::process::exit(1);
    });
    let result = patch::apply(&parsed.ops, &original).unwrap_or_else(|e| {
        eprintln!("Error applying patch: {e}");
        std::process::exit(1);
    });

    if let Some(out_path) = output_path {
        std::fs::write(out_path, &result).expect("Cannot write output file");
    } else {
        print!("{result}");
    }
}

fn run_diff_stdin(matches: &clap::ArgMatches) {
    let visual = matches.get_flag("visual");
    let fmt = matches.get_one::<String>("format").unwrap().as_str();
    let width = *matches.get_one::<usize>("width").unwrap();

    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).expect("Cannot read stdin");

    let (text_a, text_b) = input.split_once('\0').unwrap_or_else(|| {
        eprintln!("Error: --stdin expects two documents separated by a null byte (\\0)");
        std::process::exit(1);
    });

    let prep_a = diff::prepare_lines(text_a);
    let prep_b = diff::prepare_lines(text_b);
    let ops = diff::diff_lines(&prep_a, &prep_b);

    if visual || fmt == "visual" {
        format::format_visual(&ops, &prep_a, width);
    } else if fmt == "btr-context" {
        format::format_btr_context(&ops, "a", "b");
    } else {
        format::format_btr(&ops, "a", "b");
    }
}

fn run_diff(matches: &clap::ArgMatches) {
    let f1 = matches.get_one::<String>("f1").unwrap();
    let f2 = matches.get_one::<String>("f2").unwrap();
    let visual = matches.get_flag("visual");
    let fmt = matches.get_one::<String>("format").unwrap().as_str();
    let width = *matches.get_one::<usize>("width").unwrap();

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
            format::format_visual(&ops, prep_a, width);
        } else if fmt == "btr-context" {
            format::format_btr_context(&ops, f1, f2);
        } else {
            format::format_btr(&ops, f1, f2);
        }
    });
}
