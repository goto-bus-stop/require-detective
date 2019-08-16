use require_detective::detective;

fn err(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1)
}

fn main() {
    let file = match std::env::args().nth(1) {
        Some(file) => file,
        None => err(""),
    };

    let source = match std::fs::read(file) {
        Ok(source) => source,
        Err(error) => err(&error.to_string()),
    };

    let source = std::str::from_utf8(&source).unwrap();
    let result = match detective(source, &Default::default()) {
        Ok(result) => result,
        Err(error) => err(&error.to_string()),
    };

    for name in result {
        println!("{}", name);
    }
}
