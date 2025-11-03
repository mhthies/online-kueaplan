use std::io::Write;
use std::str::FromStr;

/// Ask the user interactively for some single-line value in the terminal. The user's input is
/// converted to type [T]. In case of an error, the error is printed and the user is queried again
/// and again with same prompt until the entered value is parsed successfully.
pub fn query_user<T: FromStr>(prompt: &str) -> T
where
    <T as FromStr>::Err: std::fmt::Display,
{
    query_user_and_check(prompt, |_| Ok::<(), &str>(()))
}

/// Ask the user interactively for some single-line value in the terminal. The user's input is
/// converted to type [T] and validated with the provided validation_function. In case of a parsing
/// error or validation error, the error is printed and the user is queried again and again with
/// same prompt until the entered value is valid.
pub fn query_user_and_check<T: FromStr, F, E>(prompt: &str, validation_function: F) -> T
where
    <T as FromStr>::Err: std::fmt::Display,
    F: Fn(&T) -> Result<(), E>,
    E: std::fmt::Display,
{
    loop {
        println!("{}:", prompt);
        print!("> ");
        std::io::stdout().flush().unwrap();
        let mut user_input = String::new();
        match std::io::stdin().read_line(&mut user_input) {
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
            _ => {}
        }
        let value = match user_input.trim().parse() {
            Ok(value) => value,
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        };
        match validation_function(&value) {
            Ok(()) => return value,
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
