#[macro_use]
extern crate lazy_static;
extern crate getopts;
extern crate rand;
extern crate regex;

use getopts::Options;
use rand::{thread_rng, Rng};
use regex::Regex;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::path::PathBuf;
use std::str::FromStr;

lazy_static! {
    static ref REGEX_LINE: Regex =
        Regex::from_str("^\\| (_[res]_)? ?(.+) \\| (.+) \\| (.+) \\| (.*) \\|$").unwrap();
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum WordCategory {
    Noun,
    Verb,
    AuxiliaryVerb,
    Adjective,
    Adverb,
    Preposition,
    Interjection,
    Phrase,
}

impl FromStr for WordCategory {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "m." | "f." | "n." | "pl." => Ok(WordCategory::Noun),
            "v." | "i." | "t." => Ok(WordCategory::Verb),
            "a." | "adj." => Ok(WordCategory::Adjective),
            "h." => Ok(WordCategory::AuxiliaryVerb),
            "adv." => Ok(WordCategory::Adverb),
            "prp.2" | "prp.3" | "prp.4" | "prp.3/4" => Ok(WordCategory::Preposition),
            "int." => Ok(WordCategory::Interjection),
            "R." => Ok(WordCategory::Phrase),
            _ => Err(()),
        }
    }
}

struct Word {
    word: String,
    category: WordCategory,
    line: String,
}

impl FromStr for Word {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
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
                word,
                category,
                line,
            })
            .ok_or(())
    }
}

enum InputFile {
    Path(PathBuf),
    StdIn,
}

impl InputFile {
    fn unwrap(self) -> PathBuf {
        match self {
            InputFile::Path(p) => p,
            InputFile::StdIn => panic!(),
        }
    }
}

enum SortType {
    Alphabet,
    Category,
    Random,
}

struct Config {
    input_file: InputFile,
    sort_type: SortType,
    replace: bool,
}

impl Config {
    fn from(opt_matches: &getopts::Matches) -> Result<Self, ()> {
        let (input_file, is_input_stdin) = if !opt_matches.free.is_empty() {
            if opt_matches.free[0] == "-" {
                (InputFile::StdIn, true)
            } else {
                (InputFile::Path(PathBuf::from(&opt_matches.free[0])), false)
            }
        } else {
            return Err(());
        };

        Ok(Config {
            input_file: input_file,
            sort_type: match (
                opt_matches.opt_present("alphabet"),
                opt_matches.opt_present("category"),
                opt_matches.opt_present("shuffle"),
            ) {
                (_, false, false) => SortType::Alphabet,
                (false, true, false) => SortType::Category,
                (false, false, true) => SortType::Random,
                _ => return Err(()),
            },
            replace: match (opt_matches.opt_present("replace"), is_input_stdin) {
                (b, false) => b,
                _ => return Err(()),
            },
        })
    }
}

fn print_help(program: &str, opts: &Options) {
    let brief = format!("Usage: {} FILE [options]\n\n   FILE        The words markdown file to parse for table of contents,\n               or \"-\" to read from stdin", program);
    eprint!("{}", opts.usage(&brief));
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help message")
        .optflag("a", "alphabet", "sort alphabetically (default)")
        .optflag("c", "category", "sort with category")
        .optflag("s", "shuffle", "shuffle randomly")
        .optflag("r", "replace", "replace the original file");

    let config = match Config::from(&opts.parse(&args[1..]).unwrap()) {
        Ok(c) => c,
        Err(_) => {
            print_help(&program, &opts);
            return;
        }
    };

    let mut content = String::new();
    match config.input_file {
        InputFile::StdIn => std::io::stdin().read_to_string(&mut content),
        InputFile::Path(ref p) => File::open(p).unwrap().read_to_string(&mut content),
    }.unwrap();

    let stdout = std::io::stdout();

    let mut output: Box<Write> = if config.replace {
        Box::new(File::create(&config.input_file.unwrap()).unwrap())
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

    let mut list = Vec::new();

    while !line.is_empty() && REGEX_LINE.is_match(line.trim()) {
        if let Ok(word) = Word::from_str(line.trim()) {
            list.push(word);
        } else {
            write!(output, "{}", line).unwrap();
        }
        line.clear();
        bufreader.read_line(&mut line).unwrap();
    }

    match config.sort_type {
        SortType::Random => {
            let mut rng = thread_rng();
            rng.shuffle(&mut list);
        }
        ref t => list.sort_by(|a, b| match (t, a.category.cmp(&b.category)) {
            (SortType::Alphabet, _) | (SortType::Category, Ordering::Equal) => {
                a.word.to_lowercase().cmp(&b.word.to_lowercase())
            }
            (_, ne) => ne,
        }),
    }

    for word in list.into_iter() {
        write!(output, "{}\n", word.line).unwrap();
    }

    write!(output, "{}", line).unwrap();

    for line in bufreader.lines().filter_map(|l| l.ok()) {
        write!(output, "{}\n", line).unwrap();
    }
}
