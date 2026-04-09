use crate::gs::process::expr::process_expr;
use crate::gs::process::helpers::{process_identifier, process_type};
use crate::gs::{
    AssignStmt, Block, Case, Decl, ForStmt, IfStmt, LoopBody, OnStmt, Stmt, SwitchStmt, WaitStmt,
    WhileStmt,
};
use pest::iterators::Pair;
use trainz_common::range::pair_to_range;
use trainz_parser::gs::grammar::Rule;

pub fn process_statements(pair: Pair<Rule>) -> Vec<Stmt> {
    let mut statements = vec![];
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::statements => {
                statements.extend(process_statements(inner));
            }
            Rule::statement_with_line_end
            | Rule::statement_without_line_end
            | Rule::statement_if
            | Rule::statement_while
            | Rule::statement_for
            | Rule::statement_switch
            | Rule::statement_wait
            | Rule::statement_on
            | Rule::statement_block
            | Rule::statement_label => {
                statements.push(process_stmt(inner));
            }
            Rule::semicolon | Rule::brace_open | Rule::brace_close => {}
            _ => {
                // If it's something else, try to process it as a statement if it has inner pairs
                if inner.clone().into_inner().count() > 0 {
                    statements.push(process_stmt(inner));
                }
            }
        }
    }
    statements
}

pub fn process_stmt(pair: Pair<Rule>) -> Stmt {
    let range = pair_to_range(&pair);
    match pair.as_rule() {
        Rule::statement_with_line_end | Rule::statement_without_line_end => {
            let mut inner = pair.into_inner();
            let first = inner.next().unwrap();
            process_stmt(first)
        }
        Rule::statement_label => {
            let mut inner = pair.into_inner();
            let label_pair = inner.next();
            if let Some(label_pair) = label_pair {
                let label_id = process_identifier(label_pair);
                let colon_pair = inner.next();
                let colon_range = if let Some(colon_pair) = colon_pair {
                    pair_to_range(&colon_pair)
                } else {
                    range
                };
                Stmt::Label(label_id, colon_range)
            } else {
                Stmt::Label(
                    crate::gs::Identifier {
                        name: "error".to_string(),
                        range,
                    },
                    range,
                )
            }
        }
        Rule::statement_goto => {
            let mut inner = pair.into_inner();
            let kw_range = pair_to_range(&inner.next().unwrap());
            let label_id = process_identifier(inner.next().unwrap());
            Stmt::Goto(label_id, kw_range, range)
        }
        Rule::statement_declaration => Stmt::Decl(process_decl(pair)),
        Rule::statement_return => {
            let mut inner = pair.into_inner();
            let kw_pair = inner.next().unwrap();
            let kw_range = pair_to_range(&kw_pair);
            let expr = inner.next().map(process_expr);
            Stmt::Return(expr, kw_range, range)
        }
        Rule::statement_break => {
            let kw_range = pair_to_range(&pair.into_inner().next().unwrap());
            Stmt::Break(kw_range, range)
        }
        Rule::statement_continue => {
            let kw_range = pair_to_range(&pair.into_inner().next().unwrap());
            Stmt::Continue(kw_range, range)
        }
        Rule::statement_expression | Rule::assignment_expr => Stmt::Expr(process_expr(pair)),
        Rule::statement_if => Stmt::If(process_if(pair)),
        Rule::statement_while => Stmt::While(process_while(pair)),
        Rule::statement_for => Stmt::For(process_for(pair)),
        Rule::on_body | Rule::on_body_block | Rule::statements => {
            let b_range = pair_to_range(&pair);
            Stmt::Block(Block {
                statements: process_statements(pair),
                range: b_range,
            })
        }
        Rule::statement_wait => Stmt::Wait(process_wait(pair)),
        Rule::statement_on => Stmt::On(process_on(pair)),
        Rule::statement_switch => Stmt::Switch(process_switch(pair)),
        Rule::statement_block => {
            let b_range = pair_to_range(&pair);
            let mut b_inner = pair.into_inner();
            b_inner.next(); // brace_open
            let statements_pair = b_inner.next().unwrap();
            Stmt::Block(Block {
                statements: process_statements(statements_pair),
                range: b_range,
            })
        }
        _ => unreachable!("Unexpected statement rule: {:?}", pair.as_rule()),
    }
}

fn process_decl(pair: Pair<Rule>) -> Decl {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let ty = process_type(inner.next().unwrap());
    let mut names = vec![];
    let mut values = vec![];

    let mut pending_names = vec![];
    for p in inner {
        match p.as_rule() {
            Rule::declaration_name => {
                pending_names.push(process_identifier(p));
            }
            Rule::operator_assignment
            | Rule::comma
            | Rule::line_comment
            | Rule::block_comment
            | Rule::semicolon => {
                continue;
            }
            _ => {
                if !pending_names.is_empty() {
                    let name = pending_names.pop().unwrap();
                    names.push(name);
                    values.push(process_expr(p));
                }
            }
        }
    }
    names.extend(pending_names);

    Decl {
        ty,
        names,
        values,
        range,
    }
}

fn process_if(pair: Pair<Rule>) -> IfStmt {
    let mut cond = None;
    let mut kw_if_range = None;
    let mut then_block = None;
    let mut kw_else_range = None;
    let mut else_block = None;

    let range = pair_to_range(&pair);
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::keyword_if => kw_if_range = Some(pair_to_range(&p)),
            Rule::paren_open | Rule::paren_close | Rule::line_comment | Rule::block_comment => {
                continue;
            }
            Rule::assignment_expr | Rule::statement_expression => {
                cond = Some(process_expr(p));
            }
            Rule::statement_block => {
                let b_range = pair_to_range(&p);
                let mut b_inner = p.into_inner();
                b_inner.next(); // brace_open
                let statements_pair = b_inner.next().unwrap();
                let block = Block {
                    statements: process_statements(statements_pair),
                    range: b_range,
                };
                then_block = Some(block);
            }
            Rule::statement_with_line_end
            | Rule::statement_without_line_end
            | Rule::statement_if
            | Rule::statement_while
            | Rule::statement_for
            | Rule::statement_switch
            | Rule::statement_wait
            | Rule::statement_on
            | Rule::statement_return
            | Rule::statement_label => {
                let b_range = pair_to_range(&p);
                then_block = Some(Block {
                    statements: vec![process_stmt(p)],
                    range: b_range,
                });
            }
            Rule::statement_else => {
                let e_inner = p.into_inner();
                let mut e_body = None;
                for ep in e_inner {
                    match ep.as_rule() {
                        Rule::keyword_else => kw_else_range = Some(pair_to_range(&ep)),
                        Rule::line_comment | Rule::block_comment => continue,
                        Rule::statement_block => {
                            let b_range = pair_to_range(&ep);
                            let mut b_inner = ep.into_inner();
                            b_inner.next(); // brace_open
                            let statements_pair = b_inner.next().unwrap();
                            e_body = Some(Block {
                                statements: process_statements(statements_pair),
                                range: b_range,
                            });
                        }
                        Rule::statement_with_line_end
                        | Rule::statement_without_line_end
                        | Rule::statement_if
                        | Rule::statement_while
                        | Rule::statement_for
                        | Rule::statement_switch
                        | Rule::statement_wait
                        | Rule::statement_on
                        | Rule::statement_return
                        | Rule::statement_label => {
                            let b_range = pair_to_range(&ep);
                            e_body = Some(Block {
                                statements: vec![process_stmt(ep)],
                                range: b_range,
                            });
                        }
                        _ => {}
                    }
                }
                else_block = e_body;
            }
            _ => {}
        }
    }

    if cond.is_none() || kw_if_range.is_none() || then_block.is_none() {
        return IfStmt {
            cond: cond.unwrap_or(crate::gs::Expr::Identifier(crate::gs::Identifier {
                name: "error".to_string(),
                range,
            })),
            keyword_if_range: kw_if_range.unwrap_or(range),
            then_block: then_block.unwrap_or(Block {
                statements: vec![],
                range,
            }),
            keyword_else_range: kw_else_range,
            else_block,
            range,
        };
    }

    IfStmt {
        cond: cond.unwrap(),
        keyword_if_range: kw_if_range.unwrap(),
        then_block: then_block.unwrap(),
        keyword_else_range: kw_else_range,
        else_block,
        range,
    }
}

fn process_while(pair: Pair<Rule>) -> WhileStmt {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let kw_while_range = pair_to_range(&inner.next().unwrap()); // keyword_while
    inner.next(); // paren_open
    let cond = process_expr(inner.next().unwrap());
    inner.next(); // paren_close
    let body_pair = inner.next();
    let body = match body_pair {
        None => LoopBody::Empty(crate::Range {
            start: range.end,
            end: range.end,
        }),
        Some(body_pair) => match body_pair.as_rule() {
            Rule::semicolon => LoopBody::Empty(pair_to_range(&body_pair)),
            Rule::statement_block => {
                let b_range = pair_to_range(&body_pair);
                let mut b_inner = body_pair.into_inner();
                b_inner.next(); // brace_open
                let statements_pair = b_inner.next().unwrap();
                LoopBody::Block(Block {
                    statements: process_statements(statements_pair),
                    range: b_range,
                })
            }
            _ => {
                let b_range = pair_to_range(&body_pair);
                LoopBody::Block(Block {
                    statements: vec![process_stmt(body_pair)],
                    range: b_range,
                })
            }
        },
    };

    WhileStmt {
        cond,
        keyword_while_range: kw_while_range,
        body,
        range,
    }
}

fn process_for(pair: Pair<Rule>) -> ForStmt {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();

    let kw_for_range = pair_to_range(&inner.next().unwrap()); // keyword_for
    inner.next(); // paren_open
    let init_pair = inner.next().unwrap();
    let init_range = pair_to_range(&init_pair);
    let mut init_inner = init_pair.into_inner();
    let target = process_expr(init_inner.next().unwrap());
    init_inner.next(); // operator_assignment
    let value = process_expr(init_inner.next().unwrap());
    let init = AssignStmt {
        target,
        value,
        range: init_range,
    };

    let cond = process_expr(inner.next().unwrap());

    let next = inner.next().unwrap();
    let mut step = None;
    let body_pair = if next.as_rule() != Rule::paren_close {
        step = Some(process_expr(next));
        inner.next(); // paren_close
        inner.next()
    } else {
        inner.next()
    };

    let body = match body_pair {
        None => LoopBody::Empty(crate::Range {
            start: range.end,
            end: range.end,
        }),
        Some(body_pair) => match body_pair.as_rule() {
            Rule::semicolon => LoopBody::Empty(pair_to_range(&body_pair)),
            Rule::statement_block => {
                let b_range = pair_to_range(&body_pair);
                let mut b_inner = body_pair.into_inner();
                b_inner.next(); // brace_open
                let statements_pair = b_inner.next().unwrap();
                LoopBody::Block(Block {
                    statements: process_statements(statements_pair),
                    range: b_range,
                })
            }
            _ => {
                let b_range = pair_to_range(&body_pair);
                LoopBody::Block(Block {
                    statements: vec![process_stmt(body_pair)],
                    range: b_range,
                })
            }
        },
    };

    ForStmt {
        init,
        cond,
        step,
        keyword_for_range: kw_for_range,
        body,
        range,
    }
}

fn process_wait(pair: Pair<Rule>) -> WaitStmt {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let kw_wait_range = pair_to_range(&inner.next().unwrap());
    let body_pair = inner
        .find(|p| p.as_rule() == Rule::statement_wait_block)
        .unwrap();
    let body_range = pair_to_range(&body_pair);
    WaitStmt {
        keyword_wait_range: kw_wait_range,
        body: Block {
            statements: process_statements(body_pair.into_inner().nth(1).unwrap()),
            range: body_range,
        },
        range,
    }
}

fn process_on(pair: Pair<Rule>) -> OnStmt {
    let range = pair_to_range(&pair);
    let inner = pair.into_inner();
    let mut event = None;
    let mut target = None;
    let mut identifier = None;
    let mut body = None;
    let mut kw_on_range = None;

    for p in inner {
        match p.as_rule() {
            Rule::keyword_on => kw_on_range = Some(pair_to_range(&p)),
            Rule::comma | Rule::colon => continue,
            Rule::string | Rule::string_value => {
                let sl = crate::gs::StringLiteral {
                    value: p.as_str().to_string(),
                    range: pair_to_range(&p),
                };
                if event.is_none() {
                    event = Some(sl);
                } else {
                    target = Some(sl);
                }
            }
            Rule::class_method_parameter_name => {
                identifier = Some(process_identifier(p.into_inner().next().unwrap()));
            }
            Rule::on_body | Rule::on_body_block => {
                let body_range = pair_to_range(&p);
                let statements = if p.as_rule() == Rule::on_body_block {
                    p.into_inner()
                        .find(|i| i.as_rule() == Rule::statements)
                        .map(process_statements)
                        .unwrap_or_default()
                } else {
                    process_statements(p)
                };
                body = Some(Block {
                    statements,
                    range: body_range,
                });
            }
            Rule::line_comment | Rule::block_comment => continue,
            _ => {}
        }
    }

    OnStmt {
        keyword_on_range: kw_on_range.expect("On statement missing on keyword"),
        event: event.expect("On statement missing event"),
        target: target.expect("On statement missing target"),
        identifier,
        body: body.expect("On statement missing body"),
        range,
    }
}

fn process_switch(pair: Pair<Rule>) -> SwitchStmt {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let kw_switch_range = pair_to_range(&inner.next().unwrap()); // keyword_switch
    inner.next(); // paren_open
    let expr = process_expr(inner.next().unwrap());
    inner.next(); // paren_close
    let block = inner.next().unwrap();
    let mut cases = vec![];
    let mut default = None;
    let mut kw_default_range = None;

    for p in block.into_inner() {
        let p_range = pair_to_range(&p);
        match p.as_rule() {
            Rule::statement_case => {
                let mut c_inner = p.into_inner();
                let kw_case_range = pair_to_range(&c_inner.next().unwrap()); // keyword_case
                let value = process_expr(c_inner.next().unwrap());
                c_inner.next(); // colon
                let b_pair = c_inner.next();
                if let Some(b_pair) = b_pair {
                    let b_range = pair_to_range(&b_pair);
                    cases.push(Case {
                        keyword_case_range: kw_case_range,
                        value,
                        body: Block {
                            statements: process_statements(b_pair),
                            range: b_range,
                        },
                        range: p_range,
                    });
                } else {
                    cases.push(Case {
                        keyword_case_range: kw_case_range,
                        value,
                        body: Block {
                            statements: vec![],
                            range: p_range,
                        },
                        range: p_range,
                    });
                }
            }
            Rule::statement_default => {
                let mut d_inner = p.into_inner();
                kw_default_range = Some(pair_to_range(&d_inner.next().unwrap())); // keyword_default
                d_inner.next(); // colon
                let b_pair = d_inner.next();
                if let Some(b_pair) = b_pair {
                    let b_range = pair_to_range(&b_pair);
                    default = Some(Block {
                        statements: process_statements(b_pair),
                        range: b_range,
                    });
                } else {
                    default = Some(Block {
                        statements: vec![],
                        range: p_range,
                    });
                }
            }
            _ => {}
        }
    }

    SwitchStmt {
        keyword_switch_range: kw_switch_range,
        expr,
        cases,
        keyword_default_range: kw_default_range,
        default,
        range,
    }
}
