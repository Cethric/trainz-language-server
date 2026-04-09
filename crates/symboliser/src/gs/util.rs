use trainz_ast::gs::{ClassModifier, MethodModifier};
use trainz_ast::Range;

pub(crate) fn is_method_obsolete(modifiers: &Vec<(MethodModifier, Range)>) -> bool {
    modifiers
        .iter()
        .filter_map(|(modifier, _)| match modifier {
            MethodModifier::LegacyCompatibility | MethodModifier::Obsolete(_) => Some(modifier),
            _ => None,
        })
        .count()
        > 0
}

pub(crate) fn is_class_obsolete(modifiers: &Vec<(ClassModifier, Range)>) -> bool {
    modifiers
        .iter()
        .filter_map(|(modifier, _)| match modifier {
            ClassModifier::Obsolete(_) => Some(modifier),
            _ => None,
        })
        .count()
        > 0
}
