// Helper script to find structs with both Clone derive and Atomic fields
use std::fs;
use std::path::Path;

fn main() {
    let kernel_src = "/Users/wangbiao/Desktop/project/nos/kernel/src";

    for entry in fs::read_dir(kernel_src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                let lines: Vec<&str> = content.lines().collect();

                for (i, line) in lines.iter().enumerate() {
                    if line.contains("#[derive(Clone)]") && !line.trim_start().starts_with("//") {
                        // Found a Clone derive, check next 20 lines for struct and Atomic fields
                        for j in i..=(i + 20).min(lines.len() - 1) {
                            let next_line = lines[j];
                            if next_line.contains("pub struct") && next_line.contains("Atomic") {
                                println!("Found in {}: {}", path.display(), next_line);
                            }
                        }
                    }
                }
            }
        }
    }
}