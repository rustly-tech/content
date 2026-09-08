use std::io::Read as _;

/// Returns "<byte length> <first word>".
fn summarise(line: String) -> String {
    let first = line.split_whitespace().next().unwrap_or("");
    format!("{} {}", line.len(), first)
}

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).expect("stdin");
    let line = input.trim_end_matches('\n').to_string();

    println!("{}", summarise(line));
    println!("{}", line);
}
