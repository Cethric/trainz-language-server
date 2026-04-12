use tower_lsp_server::ls_types::{
    Hover, HoverContents, MarkupContent, MarkupKind, Position, Range,
};
use trainz_ast::find::HasRange;
use trainz_ast::gs::FieldModifier;
use trainz_ast::gs::find::{
    find_field_by_id_range, find_id_at_position, find_method_by_id_range, find_param_by_id_range,
};
use trainz_ast::gs::program::Program;

#[allow(deprecated)]
#[allow(clippy::type_complexity)]
#[tracing::instrument]
pub fn trainz_hover(program: &Program, position: Position) -> Option<Hover> {
    let id = find_id_at_position(program, position)?;

    if let Some((ty, name)) = program.find_variable_declaration(&id.name, position) {
        let mut value = format!("{} {}", ty, name.name);

        if let Some(field) = find_field_by_id_range(program, name.range) {
            let is_define = field
                .modifiers
                .iter()
                .any(|m| matches!(m.0, FieldModifier::Define));

            if is_define {
                if let Some(init) = &field.initializer {
                    if let Some(init_str) = get_text_from_range(&program.src, init.range()) {
                        value = format!("{} {} = {}", ty, name.name, init_str);
                    }
                }
            } else {
                value = format!("field: {} {}", ty, name.name);
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
        let mut value = String::new();
        for (modifier, _) in &method.modifiers {
            value.push_str(&format!("{} ", modifier));
        }
        value.push_str(&format!("{} {}(", method.return_type, method.name.name));
        for (i, param) in method.params.iter().enumerate() {
            if i > 0 {
                value.push_str(", ");
            }
            value.push_str(&format!("{} {}", param.ty, param.name.name));
        }
        value.push(')');

        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(id.range),
        });
    }

    None
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
    use trainz_ast::gs::process::process_trainz_ast;
    use trainz_parser::gs::parse;

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
        let hover_i = trainz_hover(&program, pos_i).unwrap();
        if let HoverContents::Markup(markup) = hover_i.contents {
            assert_eq!(markup.value, "int i");
        }

        // Hover over 'searchStr' in 'searchStr =='
        let pos_search = Position {
            line: 5,
            character: 10,
        };
        let hover_search = trainz_hover(&program, pos_search).unwrap();
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
        let hover_field =
            trainz_hover(&program, pos_field).expect("Should find hover for m_queryResult");
        if let HoverContents::Markup(markup) = hover_field.contents {
            assert_eq!(markup.value, "field: int m_queryResult");
        }

        // Hover over 'ERROR_INVALID_STATE' in 'GetError'
        let pos_define = Position {
            line: 5,
            character: 33,
        };
        let hover_define =
            trainz_hover(&program, pos_define).expect("Should find hover for ERROR_INVALID_STATE");
        if let HoverContents::Markup(markup) = hover_define.contents {
            assert_eq!(markup.value, "int ERROR_INVALID_STATE = 2");
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
        let _pos_call = Position {
            line: 3,
            character: 4,
        };
        // This will currently return None because trainz_hover doesn't resolve method calls.
        // But if I hover over the definition, it should work.
        let pos_def = Position {
            line: 1,
            character: 14,
        };
        let hover_def =
            trainz_hover(&program, pos_def).expect("Should find hover for method definition");
        if let HoverContents::Markup(markup) = hover_def.contents {
            assert_eq!(markup.value, "public void CountTags(int pid, string s)");
        }
    }
}
