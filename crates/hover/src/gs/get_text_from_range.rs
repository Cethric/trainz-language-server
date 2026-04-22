use trainz_ast::Range;

pub fn get_text_from_range(src: &str, range: Range) -> Option<String> {
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
