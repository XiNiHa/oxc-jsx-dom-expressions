use std::collections::{hash_map::Entry, HashMap};

use oxc::{semantic::SymbolFlags, span::Atom};
use oxc_traverse::{BoundIdentifier, TraverseCtx};

pub fn jsx_text_to_str(t: &Atom) -> String {
    let mut buf = String::new();
    let replaced = t.replace('\r', "").replace('\t', " ");
    let mut lines = replaced.lines().enumerate().peekable();

    while let Some((i, mut line)) = lines.next() {
        if line.is_empty() {
            continue;
        }
        if i != 0 {
            line = line.trim_start_matches(' ');
        }
        if lines.peek().is_some() {
            line = line.trim_end_matches(' ');
        }
        if line.is_empty() {
            continue;
        }
        if i != 0 && !buf.is_empty() {
            buf.push(' ')
        }
        buf.push_str(line);
    }
    buf
}

pub fn register_import_method<'a>(
    imports: &mut HashMap<(String, String), BoundIdentifier<'a>>,
    name: &str,
    module_name: &str,
    ctx: &mut TraverseCtx<'a>,
) -> BoundIdentifier<'a> {
    match imports.entry((name.to_owned(), module_name.to_owned())) {
        Entry::Occupied(entry) => entry.get().clone(),
        Entry::Vacant(entry) => entry
            .insert(ctx.generate_uid_in_root_scope(&format!("_$${}", name), SymbolFlags::Import))
            .clone(),
    }
}
