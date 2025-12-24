//! Kernel Documentation Generator
//!
//! A tool to automatically generate comprehensive documentation from Rust source code.
//! Supports Markdown, HTML, and JSON output formats.

use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

/// Documentation generator configuration
#[derive(Debug, Clone, serde::Serialize)]
pub struct DocGeneratorConfig {
    pub input_dirs: Vec<String>,
    pub output_dir: String,
    pub format: OutputFormat,
    pub include_private: bool,
    pub include_tests: bool,
}

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum OutputFormat {
    Markdown,
    Html,
    Json,
}

/// Module documentation
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModuleDoc {
    pub path: String,
    pub name: String,
    pub description: String,
    pub types: Vec<TypeDoc>,
    pub functions: Vec<FunctionDoc>,
    pub structs: Vec<StructDoc>,
    pub traits: Vec<TraitDoc>,
    pub constants: Vec<ConstantDoc>,
}

/// Type documentation
#[derive(Debug, Clone, serde::Serialize)]
pub struct TypeDoc {
    pub name: String,
    pub description: String,
    pub type_kind: TypeKind,
}

/// Type kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum TypeKind {
    Alias,
    Enum,
    Union,
}

/// Function documentation
#[derive(Debug, Clone, serde::Serialize)]
pub struct FunctionDoc {
    pub name: String,
    pub signature: String,
    pub description: String,
    pub parameters: Vec<ParameterDoc>,
    pub return_type: String,
    pub visibility: Visibility,
}

/// Parameter documentation
#[derive(Debug, Clone, serde::Serialize)]
pub struct ParameterDoc {
    pub name: String,
    pub type_name: String,
    pub description: String,
}

/// Struct documentation
#[derive(Debug, Clone, serde::Serialize)]
pub struct StructDoc {
    pub name: String,
    pub description: String,
    pub fields: Vec<FieldDoc>,
    pub implements: Vec<String>,
    pub derives: Vec<String>,
}

/// Field documentation
#[derive(Debug, Clone, serde::Serialize)]
pub struct FieldDoc {
    pub name: String,
    pub type_name: String,
    pub description: String,
    pub visibility: Visibility,
}

/// Trait documentation
#[derive(Debug, Clone, serde::Serialize)]
pub struct TraitDoc {
    pub name: String,
    pub description: String,
    pub methods: Vec<FunctionDoc>,
    pub super_traits: Vec<String>,
}

/// Constant documentation
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConstantDoc {
    pub name: String,
    pub type_name: String,
    pub value: String,
    pub description: String,
}

/// Visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Visibility {
    Public,
    Private,
    Crate,
}

/// Documentation generator
pub struct DocGenerator {
    config: DocGeneratorConfig,
    modules: Vec<ModuleDoc>,
}

impl DocGenerator {
    pub fn new(config: DocGeneratorConfig) -> Self {
        Self {
            config,
            modules: Vec::new(),
        }
    }

    /// Generate documentation from source files
    pub fn generate(&mut self) -> Result<(), io::Error> {
        let input_dirs = self.config.input_dirs.clone();
        for input_dir in &input_dirs {
            self.scan_directory(input_dir)?;
        }

        match self.config.format {
            OutputFormat::Markdown => self.generate_markdown()?,
            OutputFormat::Html => self.generate_html()?,
            OutputFormat::Json => self.generate_json()?,
        }

        self.generate_index()?;

        Ok(())
    }

    /// Scan a directory for Rust source files
    fn scan_directory(&mut self, dir_path: &str) -> Result<(), io::Error> {
        let path = Path::new(dir_path);

        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Directory not found: {}", dir_path),
            ));
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                self.scan_directory(entry_path.to_str().unwrap())?;
            } else if entry_path.extension().map_or(false, |ext| ext == "rs") {
                self.parse_file(&entry_path)?;
            }
        }

        Ok(())
    }

    /// Parse a Rust source file
    fn parse_file(&mut self, file_path: &Path) -> Result<(), io::Error> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        let mut current_module = ModuleDoc {
            path: file_path.to_str().unwrap_or("").to_string(),
            name: Self::module_name_from_path(file_path),
            description: String::new(),
            types: Vec::new(),
            functions: Vec::new(),
            structs: Vec::new(),
            traits: Vec::new(),
            constants: Vec::new(),
        };

        let mut in_comment = false;
        let mut doc_comment = String::new();

        for line in reader.lines() {
            let line = line?;

            if line.trim().starts_with("//!") {
                current_module.description.push_str(&line.trim_start_matches("//! "));
                current_module.description.push('\n');
            } else if line.trim().starts_with("///") {
                doc_comment.push_str(&line.trim_start_matches("/// "));
                doc_comment.push('\n');
            } else if line.trim().starts_with("/*") {
                in_comment = true;
            } else if line.trim().ends_with("*/") {
                in_comment = false;
            } else if !in_comment {
                self.parse_code_line(&line, &doc_comment, &mut current_module);
                doc_comment.clear();
            }
        }

        self.modules.push(current_module);
        Ok(())
    }

    /// Parse a line of code
    fn parse_code_line(&self, line: &str, doc: &str, module: &mut ModuleDoc) {
        let line = line.trim();

        if line.starts_with("pub struct ") {
            if let Some(struct_doc) = self.parse_struct(line, doc) {
                module.structs.push(struct_doc);
            }
        } else if line.starts_with("pub enum ") {
            if let Some(enum_doc) = self.parse_enum(line, doc) {
                module.types.push(enum_doc);
            }
        } else if line.starts_with("pub fn ") || line.starts_with("pub async fn ") {
            if let Some(fn_doc) = self.parse_function(line, doc) {
                module.functions.push(fn_doc);
            }
        } else if line.starts_with("pub trait ") {
            if let Some(trait_doc) = self.parse_trait(line, doc) {
                module.traits.push(trait_doc);
            }
        } else if line.starts_with("pub const ") {
            if let Some(const_doc) = self.parse_constant(line, doc) {
                module.constants.push(const_doc);
            }
        }
    }

    /// Parse a struct definition
    fn parse_struct(&self, line: &str, doc: &str) -> Option<StructDoc> {
        let rest = line.strip_prefix("pub struct ")?.trim();
        let name = rest.split('{').next()?.split('(').next()?.split('<').next()?;

        Some(StructDoc {
            name: name.to_string(),
            description: doc.trim().to_string(),
            fields: Vec::new(),
            implements: Vec::new(),
            derives: Vec::new(),
        })
    }

    /// Parse an enum definition
    fn parse_enum(&self, line: &str, doc: &str) -> Option<TypeDoc> {
        let rest = line.strip_prefix("pub enum ")?.trim();
        let name = rest.split('{').next()?.split('(').next()?.split('<').next()?;

        Some(TypeDoc {
            name: name.to_string(),
            description: doc.trim().to_string(),
            type_kind: TypeKind::Enum,
        })
    }

    /// Parse a function definition
    fn parse_function(&self, line: &str, doc: &str) -> Option<FunctionDoc> {
        let rest = line.strip_prefix("pub ").and_then(|s| s.strip_prefix("async "))?.strip_prefix("fn ")?.trim();
        let name = rest.split('(').next()?.split('<').next()?;

        let return_type = if line.contains("->") {
            line.split("->").nth(1).unwrap_or("()").trim().to_string()
        } else {
            "()".to_string()
        };

        Some(FunctionDoc {
            name: name.to_string(),
            signature: line.to_string(),
            description: doc.trim().to_string(),
            parameters: Vec::new(),
            return_type,
            visibility: Visibility::Public,
        })
    }

    /// Parse a trait definition
    fn parse_trait(&self, line: &str, doc: &str) -> Option<TraitDoc> {
        let rest = line.strip_prefix("pub trait ")?.trim();
        let name = rest.split('{').next()?.split('(').next()?.split('<').next()?;

        Some(TraitDoc {
            name: name.to_string(),
            description: doc.trim().to_string(),
            methods: Vec::new(),
            super_traits: Vec::new(),
        })
    }

    /// Parse a constant definition
    fn parse_constant(&self, line: &str, doc: &str) -> Option<ConstantDoc> {
        let rest = line.strip_prefix("pub const ")?.trim();
        let name = rest.split(':').next()?;
        let type_name = rest.split(':').nth(1)?.split('=').next()?.trim().to_string();
        let value = rest.split('=').nth(1)?.trim_end_matches(';').trim().to_string();

        Some(ConstantDoc {
            name: name.to_string(),
            type_name,
            value,
            description: doc.trim().to_string(),
        })
    }

    /// Extract module name from file path
    fn module_name_from_path(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    /// Generate Markdown documentation
    fn generate_markdown(&self) -> Result<(), io::Error> {
        let output_dir = Path::new(&self.config.output_dir);
        fs::create_dir_all(&output_dir)?;

        for module in &self.modules {
            let file_path = output_dir.join(format!("{}.md", module.name));
            let mut file = File::create(file_path)?;

            writeln!(file, "# {}", module.name)?;
            writeln!(file)?;

            if !module.description.is_empty() {
                writeln!(file, "{}", module.description)?;
                writeln!(file)?;
            }

            if !module.constants.is_empty() {
                writeln!(file, "## Constants")?;
                for constant in &module.constants {
                    writeln!(file, "### `{}`", constant.name)?;
                    writeln!(file, "**Type:** `{}`", constant.type_name)?;
                    writeln!(file, "**Value:** `{}`", constant.value)?;
                    if !constant.description.is_empty() {
                        writeln!(file, "{}", constant.description)?;
                    }
                    writeln!(file)?;
                }
                writeln!(file)?;
            }

            if !module.types.is_empty() {
                writeln!(file, "## Types")?;
                for type_doc in &module.types {
                    writeln!(file, "### `{}`", type_doc.name)?;
                    if !type_doc.description.is_empty() {
                        writeln!(file, "{}", type_doc.description)?;
                    }
                    writeln!(file)?;
                }
                writeln!(file)?;
            }

            if !module.structs.is_empty() {
                writeln!(file, "## Structs")?;
                for struct_doc in &module.structs {
                    writeln!(file, "### `{}`", struct_doc.name)?;
                    if !struct_doc.description.is_empty() {
                        writeln!(file, "{}", struct_doc.description)?;
                    }
                    writeln!(file)?;
                }
                writeln!(file)?;
            }

            if !module.traits.is_empty() {
                writeln!(file, "## Traits")?;
                for trait_doc in &module.traits {
                    writeln!(file, "### `{}`", trait_doc.name)?;
                    if !trait_doc.description.is_empty() {
                        writeln!(file, "{}", trait_doc.description)?;
                    }
                    writeln!(file)?;
                }
                writeln!(file)?;
            }

            if !module.functions.is_empty() {
                writeln!(file, "## Functions")?;
                for fn_doc in &module.functions {
                    writeln!(file, "### `{}`", fn_doc.name)?;
                    writeln!(file, "```rust")?;
                    writeln!(file, "{}", fn_doc.signature)?;
                    writeln!(file, "```")?;
                    if !fn_doc.description.is_empty() {
                        writeln!(file, "{}", fn_doc.description)?;
                    }
                    writeln!(file)?;
                }
            }
        }

        Ok(())
    }

    /// Generate HTML documentation
    fn generate_html(&self) -> Result<(), io::Error> {
        let output_dir = Path::new(&self.config.output_dir);
        fs::create_dir_all(&output_dir)?;

        for module in &self.modules {
            let file_path = output_dir.join(format!("{}.html", module.name));
            let mut file = File::create(file_path)?;

            writeln!(file, "<!DOCTYPE html>")?;
            writeln!(file, "<html>")?;
            writeln!(file, "<head>")?;
            writeln!(file, "<meta charset='utf-8'>")?;
            writeln!(file, "<title>{}</title>", module.name)?;
            writeln!(file, "<style>")?;
            writeln!(file, "body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}")?;
            writeln!(file, "h1 {{ color: #333; border-bottom: 2px solid #007acc; }}")?;
            writeln!(file, "h2 {{ color: #444; margin-top: 30px; }}")?;
            writeln!(file, "code {{ background: #f4f4f4; padding: 2px 6px; border-radius: 3px; }}")?;
            writeln!(file, "pre {{ background: #f4f4f4; padding: 15px; border-radius: 5px; overflow-x: auto; }}")?;
            writeln!(file, "</style>")?;
            writeln!(file, "</head>")?;
            writeln!(file, "<body>")?;

            writeln!(file, "<h1>{}</h1>", module.name)?;

            if !module.description.is_empty() {
                writeln!(file, "<p>{}</p>", module.description);
            }

            for fn_doc in &module.functions {
                writeln!(file, "<h2>{}</h2>", fn_doc.name)?;
                writeln!(file, "<pre><code>{}</code></pre>", fn_doc.signature);
                if !fn_doc.description.is_empty() {
                    writeln!(file, "<p>{}</p>", fn_doc.description);
                }
            }

            writeln!(file, "</body>")?;
            writeln!(file, "</html>")?;
        }

        Ok(())
    }

    /// Generate JSON documentation
    fn generate_json(&self) -> Result<(), io::Error> {
        use serde_json::to_string_pretty;

        let output_dir = Path::new(&self.config.output_dir);
        fs::create_dir_all(&output_dir)?;

        let json_output = to_string_pretty(&self.modules)?;
        let file_path = output_dir.join("docs.json");
        let mut file = File::create(file_path)?;
        file.write_all(json_output.as_bytes())?;

        Ok(())
    }

    /// Generate index page
    fn generate_index(&self) -> Result<(), io::Error> {
        let output_dir = Path::new(&self.config.output_dir);
        let file_path = output_dir.join("index.md");
        let mut file = File::create(file_path)?;

        writeln!(file, "# NOS Kernel Documentation")?;
        writeln!(file)?;
        writeln!(file, "## Modules")?;
        writeln!(file)?;

        let mut modules = self.modules.clone();
        modules.sort_by(|a, b| a.name.cmp(&b.name));

        for module in &modules {
            writeln!(file, "- [{}]({}.md) - {}", module.name, module.name,
                module.description.lines().next().unwrap_or("No description"))?;
        }

        writeln!(file)?;
        writeln!(file, "## Statistics")?;
        writeln!(file)?;
        writeln!(file, "- Total modules: {}", self.modules.len())?;
        writeln!(file, "- Total structs: {}", self.modules.iter().map(|m| m.structs.len()).sum::<usize>())?;
        writeln!(file, "- Total functions: {}", self.modules.iter().map(|m| m.functions.len()).sum::<usize>())?;
        writeln!(file, "- Total traits: {}", self.modules.iter().map(|m| m.traits.len()).sum::<usize>())?;

        Ok(())
    }
}

fn main() {
    let config = DocGeneratorConfig {
        input_dirs: vec![
            "kernel/src".to_string(),
            "bootloader/src".to_string(),
        ],
        output_dir: "docs/generated".to_string(),
        format: OutputFormat::Markdown,
        include_private: false,
        include_tests: false,
    };

    let mut generator = DocGenerator::new(config);

    match generator.generate() {
        Ok(_) => println!("Documentation generated successfully!"),
        Err(e) => eprintln!("Error generating documentation: {}", e),
    }
}
