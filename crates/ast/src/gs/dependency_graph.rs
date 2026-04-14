use crate::gs::include::Include;
use crate::gs::program::Program;
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

pub trait ProgramResolver: Sync + Send {
    fn resolve_program(&self, path: &str) -> Option<Arc<Program>>;
}

pub fn find_cyclic_includes(
    current_path: &str,
    program: &Program,
    resolver: &dyn ProgramResolver,
) -> Vec<Include> {
    program
        .includes
        .par_iter()
        .filter_map(|include| {
            if let Some(path) = &include.path {
                let path_str = path.to_string_lossy().to_string();
                let mut visited = HashSet::new();
                let mut stack = HashSet::new();
                stack.insert(current_path.to_string());

                if is_cyclic(&path_str, resolver, &mut visited, &mut stack) {
                    return Some(include.clone());
                }
            }
            None
        })
        .collect::<Vec<Include>>()
}

fn is_cyclic(
    current_path: &str,
    resolver: &dyn ProgramResolver,
    visited: &mut HashSet<String>,
    stack: &mut HashSet<String>,
) -> bool {
    if stack.contains(current_path) {
        return true;
    }
    if visited.contains(current_path) {
        return false;
    }

    stack.insert(current_path.to_string());

    let mut cyclic = false;
    if let Some(program) = resolver.resolve_program(current_path) {
        for include in &program.includes {
            if let Some(path) = &include.path {
                let path_str = path.to_string_lossy().to_string();
                if is_cyclic(&path_str, resolver, visited, stack) {
                    cyclic = true;
                    break;
                }
            }
        }
    }

    stack.remove(current_path);
    visited.insert(current_path.to_string());
    cyclic
}

impl ProgramResolver for Program {
    fn resolve_program(&self, _path: &str) -> Option<Arc<Program>> {
        None
    }
}

pub fn get_transitive_programs(
    program: &Program,
    resolver: &dyn ProgramResolver,
) -> Vec<(String, Arc<Program>)> {
    let mut results = Vec::new();
    let mut visited = HashSet::new();
    get_transitive_programs_inner(program, resolver, &mut visited, &mut results);
    results
}

fn get_transitive_programs_inner(
    program: &Program,
    resolver: &dyn ProgramResolver,
    visited: &mut HashSet<String>,
    results: &mut Vec<(String, Arc<Program>)>,
) {
    for include in &program.includes {
        if let Some(path) = &include.path {
            let path_str = path.to_string_lossy().to_string();
            if visited.insert(path_str.clone())
                && let Some(included_program) = resolver.resolve_program(&path_str)
            {
                results.push((path_str.clone(), included_program.clone()));
                get_transitive_programs_inner(&included_program, resolver, visited, results);
            }
        }
    }
}
