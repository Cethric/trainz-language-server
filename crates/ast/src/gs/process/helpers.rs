use crate::gs::{Block, Identifier, Scope, Stmt, Type, TypeOrVoid};
use pest::iterators::Pair;
use trainz_common::range::pair_to_range;
use trainz_parser::gs::grammar::Rule;

#[tracing::instrument(skip(scopes, parent, range, variables))]
pub fn push_scope(
    scopes: &mut Vec<Scope>,
    parent: Option<usize>,
    range: crate::Range,
    variables: Vec<(Type, Identifier)>,
) -> usize {
    let id = scopes.len();
    let scope = Scope {
        id,
        parent,
        children: vec![],
        variables,
        range,
    };
    scopes.push(scope);
    if let Some(parent_id) = parent {
        scopes[parent_id].children.push(id);
    }
    id
}

#[tracing::instrument(skip(scope_id, statements, range))]
pub fn create_block(scope_id: usize, statements: Vec<Stmt>, range: crate::Range) -> Block {
    Block {
        statements,
        scope_id,
        range,
    }
}

#[tracing::instrument(skip(pair))]
pub fn process_identifier(pair: Pair<Rule>) -> Identifier {
    let name = pair.as_str().to_string();
    let range = pair_to_range(&pair);
    match pair.as_rule() {
        Rule::operator_unary_not
        | Rule::operator_unary_not_not
        | Rule::operator_unary_inverse
        | Rule::operator_unary_increment
        | Rule::operator_unary_decrement
        | Rule::operator_unary_positive
        | Rule::operator_unary_negative
        | Rule::operator_math_add
        | Rule::operator_math_subtract
        | Rule::operator_assignment => {
            // This is NOT a variable, return a dummy identifier
            // that the symboliser will ignore.
            Identifier {
                name: format!("UNKNOWN_RULE_{:?}", pair.as_rule()),
                range,
            }
        }
        _ => Identifier { name, range },
    }
}

#[tracing::instrument(skip(pair))]
pub fn process_type(pair: Pair<Rule>) -> Type {
    let range = pair_to_range(&pair);
    match pair.as_rule() {
        Rule::type_bool => Type::Bool(range),
        Rule::type_int => Type::Int(range),
        Rule::type_float => Type::Float(range),
        Rule::type_object => Type::Object(range),
        Rule::type_string => Type::String(range),
        Rule::type_identifier => Type::Named(process_identifier(pair)),
        Rule::type_array => {
            let mut inner = pair.into_inner();
            let base_type_pair = inner.next().expect("array should have base type");
            let mut ty = process_type(base_type_pair);

            while let Some(next) = inner.next() {
                if next.as_rule() == Rule::bracket_open {
                    // Skip bracket_close
                    inner.next();
                    ty = Type::Array(Box::new(ty), range);
                }
            }
            ty
        }
        _ => {
            // If it's some other rule (like class_method_parameter_type),
            // it must have an inner type.
            let inner = pair.into_inner().next().expect("type should have inner");
            let mut ty = process_type(inner);
            ty.set_range(range);
            ty
        }
    }
}

#[tracing::instrument(skip(pair))]
pub fn process_type_or_void(pair: Pair<Rule>) -> TypeOrVoid {
    let range = pair_to_range(&pair);
    match pair.as_rule() {
        Rule::type_void => TypeOrVoid::Void(range),
        Rule::type_array
        | Rule::type_bool
        | Rule::type_int
        | Rule::type_float
        | Rule::type_object
        | Rule::type_string
        | Rule::type_identifier => TypeOrVoid::Type(process_type(pair)),
        _ => {
            let inner = pair
                .into_inner()
                .next()
                .expect("type_or_void should have inner");
            match inner.as_rule() {
                Rule::type_void => TypeOrVoid::Void(range),
                _ => TypeOrVoid::Type(process_type(inner)),
            }
        }
    }
}
