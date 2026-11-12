use std::fs;
use std::path::Path;

/// Tiny first version of the Void scripting language.
pub fn run_file(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    run(&source)
}

pub fn run(source: &str) -> Result<(), String> {
    let mut variables: Vec<(String, String)> = Vec::new();

    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        if let Some(value) = line.strip_prefix("print ") {
            let output = resolve(value.trim(), &variables)?;
            println!("{}", output);
        } else if let Some(declaration) = line.strip_prefix("let ") {
            let (name, value) = declaration.split_once('=')
                .ok_or_else(|| format!("Line {}: expected 'let name = value'", index + 1))?;
            let name = name.trim();
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(format!("Line {}: invalid variable name", index + 1));
            }
            let value = resolve(value.trim(), &variables)?;
            if let Some(existing) = variables.iter_mut().find(|(key, _)| key == name) {
                existing.1 = value;
            } else {
                variables.push((name.to_string(), value));
            }
        } else {
            return Err(format!("Line {}: unknown instruction '{}'", index + 1, line));
        }
    }
    Ok(())
}

fn resolve(value: &str, variables: &[(String, String)]) -> Result<String, String> {
    let value = value.trim();
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        return Ok(value[1..value.len()-1].to_string());
    }
    if let Some((_, stored)) = variables.iter().find(|(name, _)| name == value) {
        return Ok(stored.clone());
    }
    if value.parse::<i64>().is_ok() { return Ok(value.to_string()); }
    Err(format!("Unknown value or variable: {}", value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executes_print_and_variables() {
        assert!(run("let name = \"Void\"\nprint name\nprint \"hello\"").is_ok());
    }

    #[test]
    fn rejects_unknown_instruction() {
        assert!(run("explode everything").is_err());
    }
}
