use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use tower_lsp_server::ls_types::{
    Hover, HoverContents, MarkupContent, MarkupKind, Position, Range,
};
use trainz_ast::find::HasRange;
use trainz_ast::gs::dependency_graph::ProgramResolver;
use trainz_ast::gs::find::{
    find_field_by_id_range, find_id_at_position, find_include_at_position, find_method_by_id_range,
    find_param_by_id_range, find_postfix_at_position,
};
use trainz_ast::gs::program::Program;
use trainz_ast::gs::type_eval::{ClassResolver, EvaluatedType, evaluate_expr_type};
use trainz_ast::gs::{Expr, FieldModifier, Type};

#[allow(deprecated)]
#[allow(clippy::type_complexity)]
#[tracing::instrument(skip(resolver, program_resolver))]
pub fn trainz_hover(
    program: &Program,
    resolver: &dyn ClassResolver,
    program_resolver: &dyn ProgramResolver,
    position: Position,
) -> Option<Hover> {
    if let Some(include) = find_include_at_position(program, position)
        && let Some(path) = &include.path
    {
        let path_str = path.to_string_lossy().to_string();
        if let Some(included_program) = program_resolver.resolve_program(&path_str) {
            let transitive = trainz_ast::gs::dependency_graph::get_transitive_programs(
                &included_program,
                program_resolver,
            );

            let mut classes = HashSet::<String>::new();
            for (_, p) in std::iter::once(("".to_string(), included_program)).chain(transitive) {
                for class_name in p.classes.keys() {
                    classes.insert(class_name.clone());
                }
            }

            if !classes.is_empty() {
                let mut names = classes
                    .par_iter()
                    .map(|c| format!("- {}", c))
                    .collect::<Vec<String>>();
                names.sort();
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("Includes classes:\n{}", names.join("\n")),
                    }),
                    range: Some(include.range),
                });
            }
        }
    }

    let id = find_id_at_position(program, position)?;

    if let Some((ty, name)) = program.find_variable_declaration(&id.name, position) {
        let mut value = format!("{} {}", ty, name.name);

        if let Some(field) = find_field_by_id_range(program, name.range) {
            let is_define = field
                .modifiers
                .iter()
                .any(|m| matches!(m.0, FieldModifier::Define));

            let class_prefix = if let Some(parent) = &field.parent_class {
                format!("{}::", parent)
            } else {
                "".to_string()
            };

            if is_define {
                if let Some(init) = &field.initializer
                    && let Some(init_str) = get_text_from_range(&program.src, init.range())
                {
                    value = format!("{} {}{} = {}", ty, class_prefix, name.name, init_str);
                }
            } else {
                value = format!("field: {} {}{}", ty, class_prefix, name.name);
            }
        } else if find_param_by_id_range(program, name.range).is_some() {
            value = format!("parameter: {} {}", ty, name.name);
        }

        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(id.range),
        });
    }

    if let Some(method) = find_method_by_id_range(program, id.range) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format_method_hover(method),
            }),
            range: Some(id.range),
        });
    }

    // Check if it's a method call or variable in the current context
    let current_class = trainz_ast::gs::find::find_class_at_position(program, position);
    if id.name == "me"
        && let Some(class) = current_class
    {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("class {}", class.name.name),
            }),
            range: Some(id.range),
        });
    }

    if id.name == "isclass" {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "bool isclass(object cls)".to_string(),
            }),
            range: Some(id.range),
        });
    }

    if let Some(class) = current_class {
        if let Some(field) = class.find_field(program, resolver, &id.name) {
            let class_prefix = if let Some(parent) = &field.parent_class {
                format!("{}::", parent)
            } else {
                "".to_string()
            };
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("field: {} {}{}", field.ty, class_prefix, field.name.name),
                }),
                range: Some(id.range),
            });
        }
        if let Some(methods) = class.find_method(resolver, &id.name) {
            let mut value = String::new();
            for (i, method) in methods.iter().enumerate() {
                if i > 0 {
                    value.push_str("\n\n---\n\n");
                }
                value.push_str(&format_method_hover(method));
            }
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(id.range),
            });
        }
    }

    // Check if it's a member of a postfix expression (chained call)
    if let Some((postfix_expr, idx)) = find_postfix_at_position(program, position)
        && let Expr::Postfix { expr, ops, .. } = postfix_expr
    {
        let ops_before = &ops[..idx];
        let res = if ops_before.is_empty() {
            evaluate_expr_type(
                expr,
                program,
                resolver,
                position,
                current_class,
                &HashMap::new(),
            )
        } else {
            let temp_expr = Expr::Postfix {
                expr: expr.clone(),
                ops: ops_before.to_vec(),
                range: expr.range(),
            };
            evaluate_expr_type(
                &temp_expr,
                program,
                resolver,
                position,
                current_class,
                &HashMap::new(),
            )
        };

        match res {
            Ok(EvaluatedType::Type(Type::Named(class_id))) => {
                if let Some(class) = resolver.find_class(&class_id.name) {
                    if let Some(field) = class.find_field(program, resolver, &id.name) {
                        let class_prefix = if let Some(parent) = &field.parent_class {
                            format!("{}::", parent)
                        } else {
                            "".to_string()
                        };
                        return Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: format!(
                                    "field: {} {}{}",
                                    field.ty, class_prefix, field.name.name
                                ),
                            }),
                            range: Some(id.range),
                        });
                    } else if let Some(methods) = class.find_method(resolver, &id.name) {
                        let mut value = String::new();
                        for (i, method) in methods.iter().enumerate() {
                            if i > 0 {
                                value.push_str("\n\n---\n\n");
                            }
                            value.push_str(&format_method_hover(method));
                        }
                        return Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value,
                            }),
                            range: Some(id.range),
                        });
                    }
                }
                if id.name == "isclass" {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: "bool isclass(object cls)".to_string(),
                        }),
                        range: Some(id.range),
                    });
                }
            }
            Ok(ref eval_res @ EvaluatedType::Array(..))
            | Ok(ref eval_res @ EvaluatedType::Type(Type::Array(..)))
            | Ok(ref eval_res @ EvaluatedType::Type(Type::String(..))) => {
                if id.name == "size" {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: "int size()".to_string(),
                        }),
                        range: Some(id.range),
                    });
                }

                let inner_type_str = match eval_res {
                    EvaluatedType::Array(inner, _, _) => Some(format!("{}", inner)),
                    EvaluatedType::Type(Type::Array(inner, _)) => Some(format!("{}", inner)),
                    _ => None,
                };

                if id.name == "copy"
                    && let Some(inner) = inner_type_str
                {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("{}[] copy()", inner),
                        }),
                        range: Some(id.range),
                    });
                }
            }
            _ => {}
        }
    }

    // Check if it's a class name
    if let Some(cls) = resolver.find_class(&id.name) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("class {}", cls.name.name),
            }),
            range: Some(id.range),
        });
    }

    None
}

fn format_method_hover(method: &trainz_ast::gs::MethodDef) -> String {
    let mut value = String::new();
    for (modifier, _) in &method.modifiers {
        value.push_str(&format!("{} ", modifier));
    }

    let class_prefix = if let Some(parent) = &method.parent_class {
        format!("{}::", parent)
    } else {
        "".to_string()
    };

    value.push_str(&format!(
        "{} {}{}(",
        method.return_type, class_prefix, method.name.name
    ));
    if method.void_param_range.is_some() {
        value.push_str("void");
    } else {
        for (i, param) in method.params.iter().enumerate() {
            if i > 0 {
                value.push_str(", ");
            }
            value.push_str(&format!("{} {}", param.ty, param.name.name));
        }
    }
    value.push(')');
    value
}

fn get_text_from_range(src: &str, range: Range) -> Option<String> {
    let lines: Vec<&str> = src.lines().collect();

    if range.start.line as usize >= lines.len() || range.end.line as usize >= lines.len() {
        return None;
    }

    if range.start.line == range.end.line {
        let line = lines[range.start.line as usize];
        let chars: Vec<char> = line.chars().collect();
        let start = range.start.character as usize;
        let end = range.end.character as usize;

        if start <= chars.len() && end <= chars.len() {
            return Some(chars[start..end].iter().collect());
        }
    } else {
        let mut result = String::new();
        for line_idx in range.start.line..=range.end.line {
            if let Some(line) = lines.get(line_idx as usize) {
                let chars: Vec<char> = line.chars().collect();
                if line_idx == range.start.line {
                    if (range.start.character as usize) < chars.len() {
                        result.push_str(
                            &chars[range.start.character as usize..]
                                .iter()
                                .collect::<String>(),
                        );
                    }
                } else if line_idx == range.end.line {
                    if (range.end.character as usize) <= chars.len() {
                        result.push_str(
                            &chars[..range.end.character as usize]
                                .iter()
                                .collect::<String>(),
                        );
                    }
                } else {
                    result.push_str(line);
                }
                if line_idx < range.end.line {
                    result.push('\n');
                }
            }
        }
        return Some(result);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use trainz_ast::gs::process::process_trainz_ast;
    use trainz_parser::gs::parse;

    #[test]
    fn test_trainz_hover_array_methods() {
        let code = "class Test {
    void Main() {
        int[] arr = new int[4];
        arr.size();
        arr.copy();
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'size' in 'arr.size()'
        let pos_size = Position {
            line: 3,
            character: 12,
        };
        let hover_size = trainz_hover(&program, &program, &program, pos_size)
            .expect("Should find hover for size()");
        if let HoverContents::Markup(markup) = hover_size.contents {
            assert_eq!(markup.value, "int size()");
        }

        // Hover over 'copy' in 'arr.copy()'
        let pos_copy = Position {
            line: 4,
            character: 12,
        };
        let hover_copy = trainz_hover(&program, &program, &program, pos_copy)
            .expect("Should find hover for copy()");
        if let HoverContents::Markup(markup) = hover_copy.contents {
            assert_eq!(markup.value, "int[] copy()");
        }
    }

    #[test]
    fn test_trainz_hover_nested_array_methods() {
        let code = "class Test {
    void Main() {
        int[][] nested = new int[4][2];
        nested.copy();
        nested[0].copy();
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'copy' in 'nested.copy()'
        let pos_copy1 = Position {
            line: 3,
            character: 15,
        };
        let hover_copy1 = trainz_hover(&program, &program, &program, pos_copy1)
            .expect("Should find hover for nested.copy()");
        if let HoverContents::Markup(markup) = hover_copy1.contents {
            assert_eq!(markup.value, "int[][] copy()");
        }

        // Hover over 'copy' in 'nested[0].copy()'
        let pos_copy2 = Position {
            line: 4,
            character: 18,
        };
        let hover_copy2 = trainz_hover(&program, &program, &program, pos_copy2)
            .expect("Should find hover for nested[0].copy()");
        if let HoverContents::Markup(markup) = hover_copy2.contents {
            assert_eq!(markup.value, "int[] copy()");
        }
    }

    #[test]
    fn test_trainz_hover_string_methods() {
        let code = "class Test {
    void Main() {
        string s = \"test\";
        s.size();
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'size' in 's.size()'
        let pos_size = Position {
            line: 3,
            character: 12,
        };
        let hover_size = trainz_hover(&program, &program, &program, pos_size)
            .expect("Should find hover for string.size()");
        if let HoverContents::Markup(markup) = hover_size.contents {
            assert_eq!(markup.value, "int size()");
        }
    }

    #[test]
    fn test_trainz_hover_array_indexing() {
        let code = "class Test {
    void Main() {
        int[] arr = new int[4];
        int x = arr[0];
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'arr' in 'arr[0]'
        let pos_arr = Position {
            line: 3,
            character: 16,
        };
        let hover_arr = trainz_hover(&program, &program, &program, pos_arr)
            .expect("Should find hover for arr in arr[0]");
        if let HoverContents::Markup(markup) = hover_arr.contents {
            assert_eq!(markup.value, "int[] arr");
        }
    }

    #[test]
    fn test_trainz_hover_isclass() {
        let code = "class Foo {};
class Test {
    void Main() {
        Foo f = new Foo();
        f.isclass(null);
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'isclass'
        let pos = Position {
            line: 4,
            character: 12,
        };
        let hover = trainz_hover(&program, &program, &program, pos)
            .expect("Should find hover for isclass()");
        if let HoverContents::Markup(markup) = hover.contents {
            assert_eq!(markup.value, "bool isclass(object cls)");
        }
    }

    #[test]
    fn test_trainz_hover_declaration() {
        let code = "class Test {
  public bool AlreadyThereStr(string[] strArray, string searchStr)
  {
    int i;
    for (i = 0; i < 10; i++)
      if (searchStr == strArray[i])
        return true;

    return false;
  }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'i' in 'i = 0'
        let pos_i = Position {
            line: 4,
            character: 9,
        };
        let hover_i = trainz_hover(&program, &program, &program, pos_i).unwrap();
        if let HoverContents::Markup(markup) = hover_i.contents {
            assert_eq!(markup.value, "int i");
        }

        // Hover over 'searchStr' in 'searchStr =='
        let pos_search = Position {
            line: 5,
            character: 10,
        };
        let hover_search = trainz_hover(&program, &program, &program, pos_search).unwrap();
        if let HoverContents::Markup(markup) = hover_search.contents {
            assert_eq!(markup.value, "parameter: string searchStr");
        }
    }

    #[test]
    fn test_trainz_hover_fields() {
        let code = "class Test {
  int m_queryResult = 0;
  define int ERROR_INVALID_STATE = 2;

  public int GetQueryErrorCode() { return m_queryResult; }
  public int GetError() { return ERROR_INVALID_STATE; }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'm_queryResult' in 'GetQueryErrorCode'
        let pos_field = Position {
            line: 4,
            character: 42,
        };
        let hover_field = trainz_hover(&program, &program, &program, pos_field)
            .expect("Should find hover for m_queryResult");
        if let HoverContents::Markup(markup) = hover_field.contents {
            assert_eq!(markup.value, "field: int Test::m_queryResult");
        }

        // Hover over 'ERROR_INVALID_STATE' in 'GetError'
        let pos_define = Position {
            line: 5,
            character: 33,
        };
        let hover_define = trainz_hover(&program, &program, &program, pos_define)
            .expect("Should find hover for ERROR_INVALID_STATE");
        if let HoverContents::Markup(markup) = hover_define.contents {
            assert_eq!(markup.value, "int Test::ERROR_INVALID_STATE = 2");
        }
    }

    #[test]
    fn test_trainz_hover_method() {
        let code = "class Test {
  public void CountTags(int pid, string s) { }
  void Run() {
    CountTags(1, \"test\");
  }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'CountTags' in 'Run'
        let pos_call = Position {
            line: 3,
            character: 4,
        };
        let hover_call = trainz_hover(&program, &program, &program, pos_call)
            .expect("Should find hover for method call");
        if let HoverContents::Markup(markup) = hover_call.contents {
            assert_eq!(
                markup.value,
                "public void Test::CountTags(int pid, string s)"
            );
        }

        let pos_def = Position {
            line: 1,
            character: 14,
        };
        let hover_def = trainz_hover(&program, &program, &program, pos_def)
            .expect("Should find hover for method definition");
        if let HoverContents::Markup(markup) = hover_def.contents {
            assert_eq!(
                markup.value,
                "public void Test::CountTags(int pid, string s)"
            );
        }
    }

    #[test]
    fn test_trainz_hover_static_method() {
        let code = "class Router {
    static GameObject GetCurrentThreadGameObject() { return null; }
};

class Test {
    void Main() {
        GameObject g = Router.GetCurrentThreadGameObject();
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'GetCurrentThreadGameObject' in 'Main'
        let pos_call = Position {
            line: 6,
            character: 35,
        };
        let hover_call = trainz_hover(&program, &program, &program, pos_call)
            .expect("Should find hover for static method call");
        if let HoverContents::Markup(markup) = hover_call.contents {
            assert_eq!(
                markup.value,
                "static GameObject Router::GetCurrentThreadGameObject()"
            );
        }
    }

    #[test]
    fn test_trainz_hover_inheritance() {
        let code = "class Base {
    public int baseField = 0;
    public void BaseMethod(int a) {}
};
class Derived isclass Base {
    void Main() {
        BaseMethod(1);
        int x = baseField;
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'BaseMethod' call in 'Derived::Main'
        let pos_method = Position {
            line: 6,
            character: 8,
        };
        let hover_method = trainz_hover(&program, &program, &program, pos_method)
            .expect("Should find hover for inherited method");
        if let HoverContents::Markup(markup) = hover_method.contents {
            assert_eq!(markup.value, "public void Base::BaseMethod(int a)");
        }

        // Hover over 'baseField' in 'Derived::Main'
        let pos_field = Position {
            line: 7,
            character: 16,
        };
        let hover_field = trainz_hover(&program, &program, &program, pos_field)
            .expect("Should find hover for inherited field");
        if let HoverContents::Markup(markup) = hover_field.contents {
            assert_eq!(markup.value, "field: int Base::baseField");
        }
    }

    #[test]
    fn test_trainz_hover_chained_calls() {
        let code = "class Logger {
    public Logger Log(string msg) { return me; }
    public int Count = 0;
};
class Test {
    void Main() {
        Logger l = new Logger();
        l.Log(\"a\").Log(\"b\").Count;
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over second 'Log' in 'l.Log(\"a\").Log(\"b\")'
        let pos_log2 = Position {
            line: 7,
            character: 21,
        };
        let hover_log2 = trainz_hover(&program, &program, &program, pos_log2)
            .expect("Should find hover for second Log()");
        if let HoverContents::Markup(markup) = hover_log2.contents {
            assert_eq!(markup.value, "public Logger Logger::Log(string msg)");
        }

        // Hover over 'Count' at the end of the chain
        let pos_count = Position {
            line: 7,
            character: 29,
        };
        let hover_count = trainz_hover(&program, &program, &program, pos_count)
            .expect("Should find hover for Count field");
        if let HoverContents::Markup(markup) = hover_count.contents {
            assert_eq!(markup.value, "field: int Logger::Count");
        }
    }

    #[test]
    fn test_trainz_hover_class_name() {
        let code = "class Foo {};
class Test {
    void Main() {
        Foo f = new Foo();
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'Foo' in 'Foo f'
        let pos_decl = Position {
            line: 3,
            character: 8,
        };
        let hover_decl = trainz_hover(&program, &program, &program, pos_decl)
            .expect("Should find hover for class name in declaration");
        if let HoverContents::Markup(markup) = hover_decl.contents {
            assert_eq!(markup.value, "class Foo");
        }

        // Hover over 'Foo' in 'new Foo()'
        let pos_new = Position {
            line: 3,
            character: 20,
        };
        let hover_new = trainz_hover(&program, &program, &program, pos_new)
            .expect("Should find hover for class name in new expression");
        if let HoverContents::Markup(markup) = hover_new.contents {
            assert_eq!(markup.value, "class Foo");
        }

        // Hover over 'Base' in 'class Derived isclass Base'
        let code2 = "class Base {}; class Derived isclass Base {};";
        let pairs2 = parse(code2).unwrap();
        let program2 = process_trainz_ast(pairs2, code2);
        let pos_base = Position {
            line: 0,
            character: 38,
        };
        let hover_base = trainz_hover(&program2, &program2, &program2, pos_base)
            .expect("Should find hover for superclass name");
        if let HoverContents::Markup(markup) = hover_base.contents {
            assert_eq!(markup.value, "class Base");
        }
    }

    #[test]
    fn test_trainz_hover_isclass_standalone() {
        let code = "class Foo {};
class Test {
    void Main() {
        isclass(Foo);
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'isclass'
        let pos = Position {
            line: 3,
            character: 8,
        };
        // This currently fails at find_id_at_position if we don't fix it
        let hover = trainz_hover(&program, &program, &program, pos);
        assert!(
            hover.is_some(),
            "Should find hover for standalone isclass()"
        );
        if let Some(h) = hover
            && let HoverContents::Markup(markup) = h.contents
        {
            assert_eq!(markup.value, "bool isclass(object cls)");
        }
    }

    #[test]
    fn test_trainz_hover_array_type_declaration() {
        let code = "class Foo {};
class Test {
    void Main() {
        Foo[] arr = new Foo[10];
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'Foo' in 'Foo[] arr'
        let pos_decl = Position {
            line: 3,
            character: 8,
        };
        let hover_decl = trainz_hover(&program, &program, &program, pos_decl);
        assert!(
            hover_decl.is_some(),
            "Should find hover for class name in array declaration"
        );
        if let Some(h) = hover_decl
            && let HoverContents::Markup(markup) = h.contents
        {
            assert_eq!(markup.value, "class Foo");
        }

        // Hover over 'Foo' in 'new Foo[10]'
        let pos_new = Position {
            line: 3,
            character: 24,
        };
        let hover_new = trainz_hover(&program, &program, &program, pos_new);
        assert!(
            hover_new.is_some(),
            "Should find hover for class name in new array expression"
        );
        if let Some(h) = hover_new
            && let HoverContents::Markup(markup) = h.contents
        {
            assert_eq!(markup.value, "class Foo");
        }
    }

    #[test]
    fn test_trainz_hover_field_array_type() {
        let code = "class Foo {};
class Test {
    Foo[] arr;
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'Foo' in 'Foo[] arr'
        let pos = Position {
            line: 2,
            character: 5,
        };
        let hover = trainz_hover(&program, &program, &program, pos);
        assert!(
            hover.is_some(),
            "Should find hover for class name in field array declaration"
        );
        if let Some(h) = hover
            && let HoverContents::Markup(markup) = h.contents
        {
            assert_eq!(markup.value, "class Foo");
        }
    }

    #[test]
    fn test_trainz_hover_method_types() {
        let code = "class Foo {};
class Test {
    Foo Method(Foo param) { return new Foo(); }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'Foo' in return type
        let pos_ret = Position {
            line: 2,
            character: 4,
        };
        let hover_ret = trainz_hover(&program, &program, &program, pos_ret);
        assert!(
            hover_ret.is_some(),
            "Should find hover for class name in return type"
        );
        if let Some(h) = hover_ret
            && let HoverContents::Markup(markup) = h.contents
        {
            assert_eq!(markup.value, "class Foo");
        }

        // Hover over 'Foo' in parameter type
        let pos_param = Position {
            line: 2,
            character: 15,
        };
        let hover_param = trainz_hover(&program, &program, &program, pos_param);
        assert!(
            hover_param.is_some(),
            "Should find hover for class name in parameter type"
        );
        if let Some(h) = hover_param
            && let HoverContents::Markup(markup) = h.contents
        {
            assert_eq!(markup.value, "class Foo");
        }
    }

    #[test]
    fn test_trainz_hover_me_keyword() {
        let code = "class Test {
    void Main() {
        me.Main();
    }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'me'
        let pos = Position {
            line: 2,
            character: 8,
        };
        let hover = trainz_hover(&program, &program, &program, pos)
            .expect("Should find hover for me keyword");
        if let HoverContents::Markup(markup) = hover.contents {
            assert_eq!(markup.value, "class Test");
        }
    }

    #[test]
    fn test_trainz_hover_void_parameter() {
        let code = "class Test {
    void FiremanWave(void) { }
};";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        // Hover over 'FiremanWave'
        let pos = Position {
            line: 1,
            character: 14,
        };
        let hover = trainz_hover(&program, &program, &program, pos)
            .expect("Should find hover for FiremanWave");
        if let HoverContents::Markup(markup) = hover.contents {
            assert_eq!(markup.value, "void Test::FiremanWave(void)");
        }
    }

    #[test]
    fn test_trainz_hover_include() {
        let code_a = "class A { void MethodA() {} }; include \"b.gs\"";
        let code_b = "class B { void MethodB() {} };";
        let code_main = "include \"a.gs\"";

        // Setup mock resolver
        struct MockResolver {
            programs: HashMap<String, Arc<Program>>,
        }
        impl ClassResolver for MockResolver {
            fn find_class(&self, name: &str) -> Option<trainz_ast::gs::ClassDef> {
                for p in self.programs.values() {
                    if let Some(cls) = p.classes.get(name) {
                        return Some(cls.clone());
                    }
                }
                None
            }
        }
        impl ProgramResolver for MockResolver {
            fn resolve_program(&self, path: &str) -> Option<Arc<Program>> {
                self.programs.get(path).cloned()
            }
        }

        let pairs_a = parse(code_a).unwrap();
        let mut program_a_val = process_trainz_ast(pairs_a, code_a);
        program_a_val.includes[0].path = Some(PathBuf::from("b.gs"));
        let program_a = Arc::new(program_a_val);

        let pairs_b = parse(code_b).unwrap();
        let program_b = Arc::new(process_trainz_ast(pairs_b, code_b));

        let pairs_main = parse(code_main).unwrap();
        let mut program_main = process_trainz_ast(pairs_main, code_main);
        program_main.includes[0].path = Some(PathBuf::from("a.gs"));

        let mut programs = HashMap::new();
        programs.insert("a.gs".to_string(), program_a);
        programs.insert("b.gs".to_string(), program_b);

        let resolver = MockResolver { programs };

        let pos = Position {
            line: 0,
            character: 5,
        }; // Over "include"
        let hover = trainz_hover(&program_main, &resolver, &resolver, pos)
            .expect("Should find hover for include");

        if let HoverContents::Markup(markup) = hover.contents {
            assert!(markup.value.contains("Includes classes:"));
            assert!(markup.value.contains("- A"));
            assert!(markup.value.contains("- B"));
        }
    }
}
