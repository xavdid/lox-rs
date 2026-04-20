pub struct Source {}

pub fn read_source(_soruce: &str) -> Source {
    println!("Reading source!");
    Source {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        read_source("");
    }
}
