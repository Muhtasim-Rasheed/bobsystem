use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::CompilerCtx;
use crate::rlower::Resolver;
use crate::{
    ast::{ItemKind, Program},
    diag::Diag,
    lexer::Lexer,
    parser::Parser,
    rast::{ModuleId, RProgram},
    source::{SourceFile, Span},
};

struct ModuleLoader {
    next_id: u32,
    discovered: HashMap<PathBuf, ModuleId>,
    in_progress: HashSet<PathBuf>,
    loaded: Vec<(ModuleId, String, Rc<CompilerCtx>, Program)>,
}

impl ModuleLoader {
    fn new() -> Self {
        Self {
            next_id: 0,
            discovered: HashMap::new(),
            in_progress: HashSet::new(),
            loaded: Vec::new(),
        }
    }

    fn new_module_id(&mut self) -> ModuleId {
        let id = ModuleId(self.next_id);
        self.next_id += 1;
        id
    }

    fn load(
        &mut self,
        name: &str,
        path: &Path,
        base_dir: &Path,
        importer: Option<(&Rc<CompilerCtx>, Span)>,
        root_ctx: &Rc<CompilerCtx>,
    ) -> Option<ModuleId> {
        let full_path = base_dir.join(path);
        let canonical = match full_path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                let msg = format!(
                    "couldn't find module `{name}` ({}): {e}",
                    full_path.display()
                );
                match importer {
                    Some((ctx, span)) => {
                        ctx.emit_diag(Diag::new(&ctx.source, msg).with_label(span, true));
                    }
                    None => eprintln!("error: {msg}"),
                }
                return None;
            }
        };

        if let Some(&id) = self.discovered.get(&canonical) {
            return Some(id);
        }
        if self.in_progress.contains(&canonical) {
            let msg = format!("circular import detected at {}", canonical.display());
            match importer {
                Some((ctx, span)) => {
                    ctx.emit_diag(Diag::new(&ctx.source, msg).with_label(span, true))
                }
                None => eprintln!("error: {msg}"),
            }
            return None;
        }

        let id = self.new_module_id();
        self.discovered.insert(canonical.clone(), id);
        self.in_progress.insert(canonical.clone());

        let src = match std::fs::read_to_string(&canonical) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: couldn't read {}: {e}", canonical.display());
                self.in_progress.remove(&canonical);
                return None;
            }
        };

        let file_name = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(name)
            .to_string();
        let file_ctx = match importer {
            Some((parent_ctx, _)) => Rc::new(parent_ctx.other_ctx(SourceFile::new(file_name, src))),
            None => Rc::new(root_ctx.other_ctx(SourceFile::new(file_name, src))),
        };

        let lexer = Lexer::new(&file_ctx);
        let mut parser = Parser::new(lexer);
        let mut items = Vec::new();
        while let Some(item) = parser.maybe_item() {
            items.push(item);
        }
        let program = Program { items };

        let this_dir = canonical.parent().unwrap_or(Path::new(".")).to_path_buf();
        for item in &program.items {
            if let ItemKind::Import(imported_name) = &item.inner {
                let imported_path = PathBuf::from(format!("{}.bobl", imported_name.inner));
                self.load(
                    &imported_name.inner,
                    &imported_path,
                    &this_dir,
                    Some((&file_ctx, imported_name.span)),
                    root_ctx,
                );
            }
        }

        self.in_progress.remove(&canonical);
        self.loaded.push((id, name.to_string(), file_ctx, program));
        Some(id)
    }
}

pub fn compile(entry_path: &Path) -> Option<RProgram> {
    let root_ctx = Rc::new(CompilerCtx {
        source: Rc::new(SourceFile::new("<root>", "")),
        diags: Rc::new(std::cell::RefCell::new(Vec::new())),
    });

    let mut loader = ModuleLoader::new();

    let base_dir = entry_path.parent().unwrap_or(Path::new(".")).to_path_buf();

    let file_name = entry_path.file_name()?.to_str()?;

    loader.load(file_name, Path::new(file_name), &base_dir, None, &root_ctx)?;

    let modules = loader.loaded;

    let mut resolver = Resolver::new(Rc::clone(&root_ctx));

    for (id, name, _ctx, program) in &modules {
        resolver.register_module(name, *id, program);
    }

    for (id, _name, _ctx, program) in &modules {
        resolver.register_imports(*id, program);
    }

    for (id, _name, ctx, program) in modules {
        resolver.resolve_module(id, ctx, program);
    }

    let program = resolver.into_program();

    if flush_diags(&root_ctx) {
        return None;
    }
    Some(program)
}

fn flush_diags(ctx: &CompilerCtx) -> bool {
    let diags = ctx.diags.take();
    let mut err = false;
    let mut out = String::new();
    for diag in diags {
        err |= matches!(diag.severity, crate::diag::Severity::Error);
        diag.render(&mut out);
    }
    println!("{}", out.trim());
    err
}
