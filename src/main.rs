
use jsonpath_plus::find_str;
use jsonpath_plus::error::ParseOrJsonError;

pub fn main() {
    let args = std::env::args().into_iter().collect::<Vec<_>>();
    match find_str(&args[1], &args[2]) {
        Ok(found) => println!("Found: {:?}", found.into_iter().map(|a| a.to_string()).collect::<Vec<_>>()),
        Err(ParseOrJsonError::Parse(p)) => println!("{}", p),
        Err(ParseOrJsonError::Json(j)) => println!("Error parsing provided JSON: {}", j),
    }
}
