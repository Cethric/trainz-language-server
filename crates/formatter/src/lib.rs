use trainz_ast::gs::ClassModifier;

use shadow_rs::shadow;
use trainz_ast::gs::program::Program;

shadow!(build);

pub fn format_program(program: &Program) -> String {
    let mut buffer = String::from("");

    for include in program.includes.iter() {
        buffer.push_str(&format!("include \"{}\"", include.name));
        buffer.push('\n');
    }
    buffer.push('\n');

    for klass in program.classes.iter() {
        for modifier in klass.modifiers.iter() {
            let str = match modifier.0 {
                ClassModifier::Final => String::from("final"),
                ClassModifier::Game => String::from("game"),
                ClassModifier::Static => String::from("static"),
                ClassModifier::Secured => String::from("secured"),
                ClassModifier::Obsolete(Some(value)) => format!("obsolete({})", value),
                ClassModifier::Obsolete(None) => String::from("obsolete"),
            };
            buffer.push_str(str.as_str());
            buffer.push_str(" ");
        }
        buffer.push_str(&format!(
            "class {}{} {{\n",
            klass.name.name,
            if klass.superclasses.len() > 0 {
                format!(
                    " isclass {}",
                    klass
                        .superclasses
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<&str>>()
                        .join(", ")
                )
            } else {
                "".to_string()
            }
        ));
        buffer.push('}');
        buffer.push('\n');
        buffer.push('\n');
    }

    buffer
}
