use crate::find::HasRange;

pub mod process;

#[derive(Debug, Clone)]
pub struct CommentProgram {
    pub comments: Vec<Comment>,
    pub range: crate::Range,
}

impl HasRange for CommentProgram {
    fn range(&self) -> crate::Range {
        self.range
    }
}

#[derive(Debug, Clone)]
pub enum Comment {
    LineComment(LineComment),
    BlockComment(BlockComment),
    GroupComment(GroupComment),
}

impl HasRange for Comment {
    fn range(&self) -> crate::Range {
        match self {
            Comment::LineComment(c) => c.range,
            Comment::BlockComment(c) => c.range,
            Comment::GroupComment(c) => c.range,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LineComment {
    pub text: String,
    pub range: crate::Range,
}

#[derive(Debug, Clone)]
pub struct BlockComment {
    pub text: String,
    pub range: crate::Range,
}

#[derive(Debug, Clone)]
pub struct GroupComment {
    pub comments: Vec<LineComment>,
    pub range: crate::Range,
}
