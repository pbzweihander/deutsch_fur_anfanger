#[macro_use]
extern crate lazy_static;
extern crate getopts;
extern crate regex;

use getopts::Options;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::str::FromStr;

lazy_static! {
    static ref REGEX_LINE: Regex =
        Regex::from_str("^\\| (_[res]_)? ?(.+) \\| (.+) \\| (.+) \\| (.*) \\|$").unwrap();
}

#[derive(Clone, Copy)]
enum SortType {
    Alphabet,
    Category,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum WordCategory {
    Noun,
    Verb,
    Adjective,
    Adverb,
    Preposition,
    Phrase,
}

impl FromStr for WordCategory {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "m." | "f." | "n." => Ok(WordCategory::Noun),
            "v." | "i." | "t." => Ok(WordCategory::Verb),
            "a." | "adj." => Ok(WordCategory::Adjective),
            "adv." => Ok(WordCategory::Adverb),
            "prp.2" | "prp.3" | "prp.4" | "prp.3/4" => Ok(WordCategory::Preposition),
            "R." => Ok(WordCategory::Phrase),
            _ => Err(()),
        }
    }
}

struct Word {
    sort_type: SortType,
    word: String,
    category: WordCategory,
    line: String,
}

impl PartialEq for Word {
    fn eq(&self, other: &Self) -> bool {
        self.word.eq(&other.word) && self.category.eq(&other.category)
    }
}

impl Eq for Word {}

impl Word {
    fn new(s: &str, sort_type: SortType) -> Result<Self, ()> {
        REGEX_LINE
            .captures(s)
            .map(|c| {
                let word = c.get(2).map(|s| s.as_str().to_owned());
                let category = c
                    .get(3)
                    .and_then(|s| WordCategory::from_str(s.as_str()).ok());
                (word, category, s.to_owned())
            })
            .and_then(|(a, b, c)| {
                if a.is_some() && b.is_some() {
                    Some((a.unwrap(), b.unwrap(), c))
                } else {
                    None
                }
            })
            .map(|(word, category, line)| Word {
                sort_type,
                word,
                category,
                line,
            })
            .ok_or(())
    }
}

impl PartialOrd for Word {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(match (self.sort_type, self.category.cmp(&other.category)) {
            (SortType::Alphabet, _) | (SortType::Category, Ordering::Equal) => {
                self.word.to_lowercase().cmp(&other.word.to_lowercase())
            }
            (_, ne) => ne,
        })
    }
}

impl Ord for Word {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.sort_type, self.category.cmp(&other.category)) {
            (SortType::Alphabet, _) | (SortType::Category, Ordering::Equal) => {
                self.word.to_lowercase().cmp(&other.word.to_lowercase())
            }
            (_, ne) => ne,
        }
    }
}

fn print_help(program: &str, opts: &Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    eprint!("{}", opts.usage(&brief));
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help message")
        .optflag("a", "alphabet", "sort alphabetically (default)")
        .optflag("c", "category", "sort with category")
        .optopt("o", "output", "output file (default: stdout)", "FILE");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic!(e.to_string()),
    };

    if matches.opt_present("h") {
        print_help(&program, &opts);
        return;
    }

    let input_file = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        print_help(&program, &opts);
        return;
    };

    let sort_type = if matches.opt_present("a") && matches.opt_present("c") {
        print_help(&program, &opts);
        return;
    } else if matches.opt_present("c") {
        SortType::Category
    } else {
        SortType::Alphabet
    };

    let mut file = File::open(input_file).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    drop(file);

    let stdout = std::io::stdout();

    let mut output: Box<Write> = if let Ok(Some(output_file)) = matches.opt_get::<String>("o") {
        Box::new(File::create(output_file).unwrap())
    } else {
        Box::new(stdout.lock())
    };

    let mut bufreader = BufReader::new(Cursor::new(content));
    let mut line = String::new();
    bufreader.read_line(&mut line).unwrap();

    while !line.is_empty() && !REGEX_LINE.is_match(line.trim()) {
        write!(output, "{}", line).unwrap();
        line.clear();
        bufreader.read_line(&mut line).unwrap();
    }

    write!(output, "{}", line).unwrap();
    line.clear();
    bufreader.read_line(&mut line).unwrap();
    write!(output, "{}", line).unwrap();
    line.clear();
    bufreader.read_line(&mut line).unwrap();

    let mut heap = BinaryHeap::<Word>::new();

    while !line.is_empty() && REGEX_LINE.is_match(line.trim()) {
        if let Ok(word) = Word::new(line.trim(), sort_type) {
            heap.push(word);
        } else {
            write!(output, "{}", line).unwrap();
        }
        line.clear();
        bufreader.read_line(&mut line).unwrap();
    }

    let words = heap.into_sorted_vec();

    for word in words.into_iter() {
        write!(output, "{}\n", word.line).unwrap();
    }

    write!(output, "{}", line).unwrap();

    for line in bufreader.lines().filter_map(|l| l.ok()) {
        write!(output, "{}\n", line).unwrap();
    }
}
