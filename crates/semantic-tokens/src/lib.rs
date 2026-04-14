pub mod acs_text;
pub mod comments;
pub mod gs;
pub mod legend;

use rayon::prelude::*;
use shadow_rs::shadow;
use tower_lsp_server::ls_types::{Range, SemanticToken, SemanticTokenModifier, SemanticTokenType};

shadow!(build);

#[tracing::instrument]
pub fn process_raw_tokens(
    mut raw_tokens: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>,
    src: Option<&str>,
) -> Vec<SemanticToken> {
    // Sort tokens by line and then by character
    raw_tokens.sort_by(|a, b| {
        a.0.start
            .line
            .cmp(&b.0.start.line)
            .then(a.0.start.character.cmp(&b.0.start.character))
    });

    // Mark comments immediately above declarations/definitions as DOCUMENTATION
    let mut line_has_decl = std::collections::HashMap::new();
    let mut line_is_only_comment = std::collections::HashMap::new();

    for (range, token_type, modifiers) in &raw_tokens {
        let line = range.start.line;
        if modifiers.contains(&SemanticTokenModifier::DECLARATION)
            || modifiers.contains(&SemanticTokenModifier::DEFINITION)
        {
            line_has_decl.insert(line, true);
        }

        let entry = line_is_only_comment.entry(line).or_insert(true);
        if *token_type != SemanticTokenType::COMMENT {
            *entry = false;
        }
    }

    let mut documentation_lines = std::collections::HashSet::new();
    let mut unique_lines: Vec<u32> = line_is_only_comment.keys().copied().collect();
    unique_lines.sort_by(|a, b| b.cmp(a)); // Descending

    for line in unique_lines {
        if *line_is_only_comment.get(&line).unwrap_or(&false)
            && *line_has_decl.get(&(line + 1)).unwrap_or(&false)
            || documentation_lines.contains(&(line + 1))
        {
            documentation_lines.insert(line);
        }
    }

    for (range, token_type, modifiers) in &mut raw_tokens {
        if *token_type == SemanticTokenType::COMMENT
            && documentation_lines.contains(&range.start.line)
            && !modifiers.contains(&SemanticTokenModifier::DOCUMENTATION)
        {
            modifiers.push(SemanticTokenModifier::DOCUMENTATION);
        }
    }

    let (token_types, token_modifiers) = legend::get_legend();

    let mut tokens = vec![];
    let mut last_line = 0;
    let mut last_start = 0;

    let line_lengths: Option<Vec<u32>> = src.map(|s| {
        s.split('\n')
            .map(|l| {
                l.trim_end_matches('\r')
                    .chars()
                    .map(|c| c.len_utf16() as u32)
                    .sum::<u32>()
            })
            .collect()
    });

    let mut expanded_raw_tokens = vec![];
    for (range, token_type, modifiers) in raw_tokens {
        if range.end.line == range.start.line {
            expanded_raw_tokens.push((range, token_type, modifiers));
        } else if let Some(lengths) = &line_lengths {
            // Split multi-line token
            for line_num in range.start.line..=range.end.line {
                let start_char = if line_num == range.start.line {
                    range.start.character
                } else {
                    0
                };
                let end_char = if line_num == range.end.line {
                    range.end.character
                } else {
                    *lengths.get(line_num as usize).unwrap_or(&0)
                };

                if end_char > start_char {
                    expanded_raw_tokens.push((
                        Range {
                            start: tower_lsp_server::ls_types::Position {
                                line: line_num,
                                character: start_char,
                            },
                            end: tower_lsp_server::ls_types::Position {
                                line: line_num,
                                character: end_char,
                            },
                        },
                        token_type.clone(),
                        modifiers.clone(),
                    ));
                }
            }
        } else {
            // Fallback: just push as multi-line (which will be length 0)
            expanded_raw_tokens.push((range, token_type, modifiers));
        }
    }

    for (range, token_type, modifiers) in expanded_raw_tokens {
        let type_idx = token_types
            .par_iter()
            .position_first(|t| *t == token_type)
            .unwrap_or(0) as u32;

        let mut modifiers_bitset = 0;
        for modifier in modifiers {
            if let Some(pos) = token_modifiers
                .par_iter()
                .position_first(|m| *m == modifier)
            {
                modifiers_bitset |= 1 << pos;
            }
        }

        let line = range.start.line;
        let start = range.start.character;
        if range.end.line < range.start.line
            || (range.end.line == range.start.line && range.end.character < range.start.character)
        {
            panic!(
                "Invalid range: {:?} to {:?}, token_type: {:?}",
                range.start, range.end, token_type
            );
        }
        let length = if range.end.line == range.start.line {
            range.end.character - range.start.character
        } else {
            0
        };

        if length == 0
            && range.end.line == range.start.line
            && range.end.character == range.start.character
        {
            continue;
        }

        let delta_line = line - last_line;
        let delta_start = if delta_line == 0 {
            start - last_start
        } else {
            start
        };

        tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: type_idx,
            token_modifiers_bitset: modifiers_bitset,
        });

        last_line = line;
        last_start = start;
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp_server::ls_types::{Position, Range, SemanticTokenType};

    #[test]
    fn test_process_raw_tokens_merging() {
        // Source 1 (e.g. gs or acs_text)
        let source_1_tokens = vec![
            (
                Range {
                    start: Position {
                        line: 0,
                        character: 4,
                    },
                    end: Position {
                        line: 0,
                        character: 10,
                    },
                },
                SemanticTokenType::KEYWORD,
                vec![],
            ),
            (
                Range {
                    start: Position {
                        line: 2,
                        character: 8,
                    },
                    end: Position {
                        line: 2,
                        character: 12,
                    },
                },
                SemanticTokenType::VARIABLE,
                vec![],
            ),
        ];

        // Source 2 (e.g. comments)
        let source_2_tokens = vec![
            (
                Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 3,
                    },
                },
                SemanticTokenType::COMMENT,
                vec![],
            ),
            (
                Range {
                    start: Position {
                        line: 1,
                        character: 4,
                    },
                    end: Position {
                        line: 1,
                        character: 14,
                    },
                },
                SemanticTokenType::COMMENT,
                vec![],
            ),
        ];

        let mut all_tokens = source_1_tokens;
        all_tokens.extend(source_2_tokens);

        let processed = process_raw_tokens(all_tokens, None);

        assert_eq!(processed.len(), 4);

        // First token: line 0, char 0 (COMMENT, length 3)
        assert_eq!(processed[0].delta_line, 0);
        assert_eq!(processed[0].delta_start, 0);
        assert_eq!(processed[0].length, 3);
        // We don't assert token_type specifically since it depends on legend order, but we assert it exists.

        // Second token: line 0, char 4 (KEYWORD, length 6)
        assert_eq!(processed[1].delta_line, 0);
        assert_eq!(processed[1].delta_start, 4); // 4 - 0 = 4
        assert_eq!(processed[1].length, 6);

        // Third token: line 1, char 4 (COMMENT, length 10)
        assert_eq!(processed[2].delta_line, 1); // 1 - 0 = 1
        assert_eq!(processed[2].delta_start, 4);
        assert_eq!(processed[2].length, 10);

        // Fourth token: line 2, char 8 (VARIABLE, length 4)
        assert_eq!(processed[3].delta_line, 1); // 2 - 1 = 1
        assert_eq!(processed[3].delta_start, 8);
        assert_eq!(processed[3].length, 4);
    }

    #[test]
    fn test_process_raw_tokens_documentation_modifier() {
        let tokens = vec![
            (
                Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 5,
                    },
                },
                SemanticTokenType::COMMENT,
                vec![],
            ),
            (
                Range {
                    start: Position {
                        line: 1,
                        character: 0,
                    },
                    end: Position {
                        line: 1,
                        character: 5,
                    },
                },
                SemanticTokenType::COMMENT,
                vec![],
            ),
            (
                Range {
                    start: Position {
                        line: 2,
                        character: 0,
                    },
                    end: Position {
                        line: 2,
                        character: 5,
                    },
                },
                SemanticTokenType::CLASS,
                vec![SemanticTokenModifier::DECLARATION],
            ),
            (
                Range {
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 4,
                        character: 5,
                    },
                },
                SemanticTokenType::COMMENT,
                vec![],
            ),
            (
                Range {
                    start: Position {
                        line: 5,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 5,
                    },
                },
                SemanticTokenType::VARIABLE,
                vec![], // No declaration modifier
            ),
        ];

        let processed = process_raw_tokens(tokens, None);

        // Expected modifiers for DOCUMENTATION
        let (_, token_modifiers) = legend::get_legend();
        let doc_modifier_bit = 1
            << token_modifiers
                .par_iter()
                .position_first(|m| *m == SemanticTokenModifier::DOCUMENTATION)
                .unwrap();

        // Line 0 and Line 1 comments should have DOCUMENTATION bitset since they immediately precede line 2 (DECLARATION)
        assert_ne!(
            processed[0].token_modifiers_bitset & doc_modifier_bit,
            0,
            "Line 0 comment should be DOCUMENTATION"
        );
        assert_ne!(
            processed[1].token_modifiers_bitset & doc_modifier_bit,
            0,
            "Line 1 comment should be DOCUMENTATION"
        );

        // Line 4 comment should NOT have DOCUMENTATION bitset since line 5 is not a DECLARATION
        assert_eq!(
            processed[3].token_modifiers_bitset & doc_modifier_bit,
            0,
            "Line 4 comment should not be DOCUMENTATION"
        );
    }
}
