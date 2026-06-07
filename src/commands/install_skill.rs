use std::fs;
use std::path::Path;
use crate::error::TelepromptError;

const SKILL_CONTENT: &str = include_str!("../../SKILL.md");

pub fn run() -> Result<(), TelepromptError> {
    let dest_path = Path::new("TELEPROMPT_SKILL.md");
    
    fs::write(dest_path, SKILL_CONTENT)
        .map_err(TelepromptError::Io)?;
        
    println!("✔ Successfully installed AI Agent skill instructions as 'TELEPROMPT_SKILL.md' in the current directory.");
    Ok(())
}
