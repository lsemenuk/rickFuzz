extern crate debugger;

use debugger::Debugger;
use std::io::Write;
use std::process;
use std::fs;

// This is the wrapper for the input we
// want to modify and feed to the target program
struct Corpus {
    image: Vec<u8>,
    seed: usize,
}

impl Corpus {
    // Return a new initialized corpus to mutate
    fn new() -> Result<Corpus, Box<dyn std::error::Error>> {
        let image = fs::read("corpus.jpg")?;
        let seed = 0x1337fe44;

        Ok(Corpus {
            image,
            seed
        })
    }

    // Get a random number using xorshift
    fn rand(&mut self) -> usize {
        let mut seed = self.seed;
        seed ^= seed << 13;
        seed ^= seed >> 7;  
        seed ^= seed << 17;
        self.seed = seed;
        seed
    }
    
    // Mutate corpus by flipping bytes
    fn mutate(&mut self) {
        let rand_ind = self.rand() % (self.image.len() - 1);
        // This is a ghetto and crappy way to make
        // sure we preserve the jpeg header while
        // doing byte flipping memes
        if rand_ind > 2 {
            let rand_byte = self.rand() % 255;
            self.image[rand_ind] = rand_byte as u8; 
            println!("Flipping byte at: {} to: {}", rand_ind, rand_byte);
        }
    }

    // Dump the ranodmized image to disk so we can run
    // the next iteration of djpeg
    fn dump(&self) {
        let mut file = fs::File::create("input_corpus.jpg")
            .expect("Error creating crash dump file.");
        file.write(&self.image)
            .expect("Error writing crash dump file.");

    }
}

// Our fuzzer will be comprised of a 
// corpus and number of crashes until
// we create coverage guidance which will most
// likely result in a debugger being added to this struct
struct Fuzzer {
    corpus: Corpus,
    crashes: usize,
}

impl Fuzzer {
    // Return a fuzzer with an initialized corpus to mutate
    // and setup directory to hold crashdumps
    fn new() -> Result<Fuzzer, Box<dyn std::error::Error>> {
        let corpus = Corpus::new()?;
        let crashes: usize = 0;

        // Do not want to err if dir already exists
        fs::create_dir_all("crash_dumps")?;

        Ok(Fuzzer {
            corpus,
            crashes,
            
        })
    }

    // We will write images that cause crashes to
    // the <root directory of this project>/crash_dumps
    fn crash_dump(&self) {
        let mut file = fs::File::create(format!("crash_dumps/crash_corpus_{}.jpg", 
                                        self.crashes.to_string(),))
                                        .expect("Error creating crash dump file");
        file.write(&self.corpus.image)
            .expect("Error writing crash dump image file.");
    }

    // Continuously dump a new mutated jpg and run the image parser
    // if we record a crash then we save input that caused the crash
    // as well.
    fn fuzz(&mut self) {
        loop {
            let status = process::Command::new("./djpeg")
                                            .arg("input_corpus.jpg")
                                            .status()
                                            .expect("Failed to get return value of program");
            // If we crash increment crash counter and
            // write out crash dump
            if !status.success() {
                println!("GOT A CRASH!!!!!!!");
                self.crashes += 1;
                self.crash_dump();
            }
            
            // Mutate image and dump it to disk
            // this is probably a huge time sink
            // might be worth looking into keeping 
            // a memfd or something
            self.corpus.mutate();
            self.corpus.dump();
        }
    }
}

fn main() {
    let mut fuzzer = Fuzzer::new().unwrap_or_else(|err| {
        eprintln!("Problem initializing the fuzzer: {}", err);
        process::exit(1);
    });

    fuzzer.fuzz();

}
