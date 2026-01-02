use std::env;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::process;

use rawk::{Interpreter, Lexer, Parser};

fn main() {
    let args: Vec<String> = env::args().collect();

    match run(&args[1..]) {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("rawk: {}", e);
            process::exit(2);
        }
    }
}

fn run(args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
    let mut field_separator = " ".to_string();
    let mut program_source: Option<String> = None;
    let mut input_files: Vec<String> = Vec::new();
    let mut variables: Vec<(String, String)> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if arg == "--help" || arg == "-h" {
            print_help();
            return Ok(0);
        }

        if arg == "--version" {
            println!("rawk {}", env!("CARGO_PKG_VERSION"));
            return Ok(0);
        }

        if arg == "-F" {
            i += 1;
            if i >= args.len() {
                return Err("option -F requires an argument".into());
            }
            field_separator = args[i].clone();
        } else if let Some(fs) = arg.strip_prefix("-F") {
            field_separator = fs.to_string();
        } else if arg == "-v" {
            i += 1;
            if i >= args.len() {
                return Err("option -v requires an argument".into());
            }
            let var_assign = &args[i];
            if let Some((name, value)) = var_assign.split_once('=') {
                variables.push((name.to_string(), value.to_string()));
            } else {
                return Err(format!("invalid variable assignment: {}", var_assign).into());
            }
        } else if arg == "-f" {
            i += 1;
            if i >= args.len() {
                return Err("option -f requires an argument".into());
            }
            let script_file = &args[i];
            program_source = Some(fs::read_to_string(script_file)?);
        } else if arg == "--" {
            // End of options
            i += 1;
            input_files.extend(args[i..].iter().cloned());
            break;
        } else if arg.starts_with('-') && arg != "-" {
            return Err(format!("unknown option: {}", arg).into());
        } else if program_source.is_none() {
            // First non-option argument is the program
            program_source = Some(arg.clone());
        } else {
            // Rest are input files
            input_files.push(arg.clone());
        }

        i += 1;
    }

    let program_source = program_source.ok_or("no program provided")?;

    // Parse the program
    let mut lexer = Lexer::new(&program_source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;

    // Create interpreter
    let mut interpreter = Interpreter::new(&program);

    // Set field separator
    interpreter.set_fs(&field_separator);

    // Set ARGC and ARGV (ARGV[0] is "awk", ARGV[1...] are input files)
    let mut argv = vec!["awk".to_string()];
    argv.extend(input_files.iter().cloned());
    interpreter.set_args(argv);

    // Set variables
    for (name, value) in &variables {
        interpreter.set_variable(name, value);
    }

    // Prepare output
    let stdout = io::stdout();
    let mut output = stdout.lock();

    // Prepare inputs
    let exit_code = if input_files.is_empty() {
        // Read from stdin
        let stdin = io::stdin();
        let inputs = vec![BufReader::new(stdin.lock())];
        interpreter.run(inputs, &mut output)?
    } else {
        // Read from files
        let mut exit_code = 0;
        for filename in &input_files {
            if filename == "-" {
                let stdin = io::stdin();
                let inputs = vec![BufReader::new(stdin.lock())];
                exit_code = interpreter.run(inputs, &mut output)?;
            } else {
                let file = File::open(filename)?;
                let inputs = vec![BufReader::new(file)];
                exit_code = interpreter.run(inputs, &mut output)?;
            }
        }
        exit_code
    };

    Ok(exit_code)
}

fn print_help() {
    println!(
        r#"Usage: rawk [OPTIONS] 'program' [file ...]
       rawk [OPTIONS] -f progfile [file ...]

A 100% POSIX-compatible AWK implementation in Rust.

Options:
  -F fs          Set the field separator to fs
  -v var=val     Assign value to variable before execution
  -f progfile    Read the AWK program from file
  --posix        Strict POSIX mode (no extensions)
  --version      Print version information
  --help         Print this help message

Examples:
  rawk '{{ print $1 }}' file.txt
  rawk -F: '{{ print $1 }}' /etc/passwd
  rawk 'BEGIN {{ print "Hello" }}'
  rawk '/pattern/ {{ print }}' file.txt
"#
    );
}
