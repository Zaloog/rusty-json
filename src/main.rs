use std::env::args;
use std::fs;
use std::path::{Path, PathBuf};

mod parser;
use parser::{parse_tokens, read_file, tokenize};

fn test_file(path: &Path) -> (bool, String) {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let should_pass = file_name.starts_with("pass");
    
    match read_file(path) {
        Ok(content) => {
            match tokenize(content) {
                Ok(tokens) => {
                    match parse_tokens(tokens) {
                        Ok(_) => {
                            if should_pass {
                                (true, format!("✓ {}: PASS (expected pass)", file_name))
                            } else {
                                (false, format!("✗ {}: FAIL (expected fail, but passed)", file_name))
                            }
                        }
                        Err(e) => {
                            if should_pass {
                                (false, format!("✗ {}: FAIL (expected pass, error: {})", file_name, e))
                            } else {
                                (true, format!("✓ {}: PASS (expected fail, got: {})", file_name, e))
                            }
                        }
                    }
                }
                Err(e) => {
                    if should_pass {
                        (false, format!("✗ {}: FAIL (tokenize error: {})", file_name, e))
                    } else {
                        (true, format!("✓ {}: PASS (expected fail, got: {})", file_name, e))
                    }
                }
            }
        }
        Err(e) => (false, format!("✗ {}: ERROR reading file: {}", file_name, e)),
    }
}

fn main() {
    let arguments: Vec<String> = args().collect();
    
    if arguments.len() < 2 {
        eprintln!("Usage: {} <file_or_directory>", arguments[0]);
        std::process::exit(1);
    }
    
    let path = Path::new(&arguments[1]);
    
    // If it's a file, parse it and exit
    if path.is_file() {
        match read_file(path) {
            Ok(content) => {
                match tokenize(content) {
                    Ok(tokens) => {
                        match parse_tokens(tokens) {
                            Ok(_) => std::process::exit(0),
                            Err(e) => {
                                eprintln!("Parse error: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Tokenize error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("File read error: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    // Otherwise, treat it as a directory with test cases
    if !path.is_dir() {
        eprintln!("Error: {} is not a file or directory", path.display());
        std::process::exit(1);
    }
    
    let test_dir = path;
    println!("Running JSON parser tests from: {}\n", test_dir.display());
    
    let mut entries: Vec<PathBuf> = fs::read_dir(test_dir)
        .expect("Failed to read directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();
    
    entries.sort();
    
    let mut passed = 0;
    let mut failed = 0;
    
    for path in entries {
        let (success, message) = test_file(&path);
        println!("{}", message);
        
        if success {
            passed += 1;
        } else {
            failed += 1;
        }
    }
    
    println!("\n═══════════════════════════════════════");
    println!("Test Results: {} passed, {} failed", passed, failed);
    println!("═══════════════════════════════════════");
    
    if failed > 0 {
        std::process::exit(1);
    }
}
