use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    file: String,
}

fn main() {
    println!("Hello, world!");
    let args = Args::parse();

    dbg!(args);
}
