use clap::{Arg, Command};

fn main() {
    let matches = Command::new("btr-diff")
        .arg(Arg::new("f1").required(true))
        .arg(Arg::new("f2").required(true))
        .get_matches();
    let f1 = matches.get_one::<String>("f1").expect("Missing argument");
    let f2 = matches.get_one::<String>("f2").expect("Missing argument");

    let mut result_vec = Vec::new();
    std::thread::scope(|s| {
        let handles = [f1, f2].map(|f| s.spawn(move || std::fs::read_to_string(f)));
        for h in handles {
            result_vec.push(
                h.join()
                    .expect("thread panicked")
                    .expect("Cannot read this file."),
            );
        }
    });

    for r in result_vec {
        println!("Hello, world! {r}");
    }
}
