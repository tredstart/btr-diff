use clap::{Arg, Command};

fn main() {
    let matches = Command::new("btr-diff")
        .arg(Arg::new("f1").required(true))
        .arg(Arg::new("f2").required(true))
        .get_matches();
    let f1 = matches.get_one::<String>("f1").expect("Missing argument");
    let f2 = matches.get_one::<String>("f2").expect("Missing argument");
    println!("Hello, world! {f1} {f2}");
}
