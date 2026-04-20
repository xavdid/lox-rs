pub type Source = ();

pub fn read_source() -> Source {
    println!("Reading source!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        read_source();
    }
}
