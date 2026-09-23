use crate::lex::LexError;
use crate::parse::{ParseError, Program};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum IncludeError {
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    Lex {
        path: PathBuf,
        error: LexError,
    },
    Parse {
        path: PathBuf,
        error: ParseError,
    },
    CircularInclude {
        path: PathBuf,
    },
}

impl std::fmt::Display for IncludeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncludeError::Io { path, error } => {
                write!(f, "couldn't read {}: {error}", path.display())
            }
            IncludeError::Lex { path, error } => {
                write!(f, "{}: {error}", path.display())
            }
            IncludeError::Parse { path, error } => {
                write!(f, "{}: {error}", path.display())
            }
            IncludeError::CircularInclude { path } => {
                write!(f, "circular include detected at {}", path.display())
            }
        }
    }
}

impl std::error::Error for IncludeError {}

pub fn resolve_includes(program: &mut Program, own_dir: &Path) -> Result<(), IncludeError> {
    let mut in_progress = HashSet::new();
    let mut done = HashSet::new();
    splice(program, own_dir, &mut in_progress, &mut done)
}

fn splice(
    program: &mut Program,
    base_dir: &Path,
    in_progress: &mut HashSet<PathBuf>,
    done: &mut HashSet<PathBuf>,
) -> Result<(), IncludeError> {
    let includes = std::mem::take(&mut program.includes);

    for rel_path in includes {
        let full_path = base_dir.join(&rel_path);
        let canonical = full_path.canonicalize().map_err(|e| IncludeError::Io {
            path: full_path.clone(),
            error: e,
        })?;

        if done.contains(&canonical) {
            continue;
        }
        if in_progress.contains(&canonical) {
            return Err(IncludeError::CircularInclude { path: canonical });
        }

        let src = std::fs::read_to_string(&canonical).map_err(|e| IncludeError::Io {
            path: canonical.clone(),
            error: e,
        })?;
        let tokens = crate::lex::lex(&src).map_err(|e| IncludeError::Lex {
            path: canonical.clone(),
            error: e,
        })?;
        let mut included = crate::parse::parse(&tokens).map_err(|e| IncludeError::Parse {
            path: canonical.clone(),
            error: e,
        })?;

        in_progress.insert(canonical.clone());
        let included_dir = canonical.parent().unwrap_or(Path::new(".")).to_path_buf();
        splice(&mut included, &included_dir, in_progress, done)?;
        in_progress.remove(&canonical);
        done.insert(canonical.clone());

        program.functions.extend(included.functions);
    }

    Ok(())
}
