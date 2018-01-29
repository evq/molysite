extern crate molysite;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use molysite::hcl::parse_hcl;

fn main() {
    if let Some(path) = env::args().nth(1) {
        // Create a path to the desired file
        let path = Path::new(&path);
        let display = path.display();

        // Open the path in read-only mode, returns `io::Result<File>`
        let mut file = match File::open(&path) {
            // The `description` method of `io::Error` returns a string that
            // describes the error
            Err(why) => panic!("couldn't open {}: {}", display,
                                                       why.description()),
            Ok(file) => file,
        };

        // Read the file contents into a string, returns `io::Result<usize>`
        let mut s = String::new();
        match file.read_to_string(&mut s) {
            Err(why) => panic!("couldn't read {}: {}", display,
                                                       why.description()),
            Ok(_) => {
                let parsed = parse_hcl(&s);
                print!("{}\n", parsed.unwrap());
            }
        }
    }
}
