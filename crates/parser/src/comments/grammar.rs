use pest_derive::Parser;

pub mod gs {
    use super::*;
    #[derive(Parser)]
    #[grammar = "comments/gs_comments.pest"]
    pub struct GsCommentsParser;
}

pub mod acs_text {
    use super::*;
    #[derive(Parser)]
    #[grammar = "comments/acs_text_comments.pest"]
    pub struct AcsTextCommentsParser;
}

#[cfg(test)]
mod tests {
    use super::acs_text::{AcsTextCommentsParser, Rule as AcsTextRule};
    use super::gs::{GsCommentsParser, Rule as GsRule};
    use pest::Parser;

    #[test]
    fn test_gs_line_comment() {
        let input = "// line comment";
        let parse_result = GsCommentsParser::parse(GsRule::line_comment, input);
        assert!(parse_result.is_ok());
        let pair = parse_result.unwrap().next().unwrap();
        assert_eq!(pair.as_rule(), GsRule::line_comment);
        assert_eq!(pair.as_str(), "// line comment");
    }

    #[test]
    fn test_gs_block_comment() {
        let input = "/* block comment */";
        let parse_result = GsCommentsParser::parse(GsRule::block_comment, input);
        assert!(parse_result.is_ok());
        let pair = parse_result.unwrap().next().unwrap();
        assert_eq!(pair.as_rule(), GsRule::block_comment);
        assert_eq!(pair.as_str(), "/* block comment */");
    }

    #[test]
    fn test_acs_text_line_comment() {
        let input = "; acs_text style comment";
        let parse_result = AcsTextCommentsParser::parse(AcsTextRule::line_comment, input);
        assert!(parse_result.is_ok());
        let pair = parse_result.unwrap().next().unwrap();
        assert_eq!(pair.as_rule(), AcsTextRule::line_comment);
        assert_eq!(pair.as_str(), "; acs_text style comment");
    }

    #[test]
    fn test_acs_text_no_gs_comment() {
        let input = "// gs comment";
        let parse_result = AcsTextCommentsParser::parse(AcsTextRule::line_comment, input);
        assert!(parse_result.is_err());
    }

    #[test]
    fn test_gs_no_acs_text_comment() {
        let input = "; acs_text comment";
        let parse_result = GsCommentsParser::parse(GsRule::line_comment, input);
        assert!(parse_result.is_err());
    }

    #[test]
    fn test_gs_comments_in_strings() {
        let input = r#"
            out_description = out_description
                        + "Speed Units: <a href=live://property/l_useMetric>" + metric_label + "</a><br>"
                        + "Normal (Track) Speed: <a href=live://property/s_normalValue>" + speed_limit_normal + "</a><br>"
                        + "Medium (Diverge) Speed: <a href=live://property/s_mediumValue>" + speed_limit_medium + "</a><br>"
                        + "Low (Subsidiary) Speed: <a href=live://property/s_lowValue>" + speed_limit_low + "</a><br>"
                        + "<br>A value of '-1' will mean the limit is ignored.<br>"
                        + "<br>Ranger_51 - TrackSpeedTarget: build " + build_version + " (" + build_date + ")";
        "#;
        let parse_result = GsCommentsParser::parse(GsRule::comment_program, input);
        assert!(parse_result.is_ok());
        let pairs = parse_result.unwrap();
        // Check if any line_comment or block_comment was found
        for pair in pairs {
            for inner in pair.into_inner() {
                if inner.as_rule() == GsRule::line_comment
                    || inner.as_rule() == GsRule::block_comment
                {
                    panic!("Found comment inside string area: {}", inner.as_str());
                }
            }
        }
    }

    #[test]
    fn test_acs_text_comments_in_strings() {
        let input = r#"
            description "This is a ; semicolon in string"
            ; this is a comment
        "#;
        let parse_result = AcsTextCommentsParser::parse(AcsTextRule::comment_program, input);
        assert!(parse_result.is_ok());
        let pairs = parse_result.unwrap();
        let mut found_comment = false;
        for pair in pairs {
            for inner in pair.into_inner() {
                if inner.as_rule() == AcsTextRule::line_comment {
                    assert_eq!(inner.as_str(), "; this is a comment");
                    found_comment = true;
                }
            }
        }
        assert!(found_comment);
    }
}
