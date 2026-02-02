//! Macro field selection for form navigation.
//!
//! This module provides the `MacroField` enum and its navigation methods
//! for cycling through macro creation form fields.

/// Field selection for macro form navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroField {
    /// Macro name field
    Name,
    /// Macro definition field (the SPL expression)
    Definition,
    /// Arguments field (comma-separated list)
    Args,
    /// Description field
    Description,
    /// Disabled toggle field
    Disabled,
    /// IsEval toggle field (whether definition is an eval expression)
    IsEval,
}

impl MacroField {
    /// Get the next field in the form (cycles through all fields).
    pub fn next(self) -> Self {
        match self {
            MacroField::Name => MacroField::Definition,
            MacroField::Definition => MacroField::Args,
            MacroField::Args => MacroField::Description,
            MacroField::Description => MacroField::Disabled,
            MacroField::Disabled => MacroField::IsEval,
            MacroField::IsEval => MacroField::Name,
        }
    }

    /// Get the previous field in the form (cycles through all fields).
    pub fn previous(self) -> Self {
        match self {
            MacroField::Name => MacroField::IsEval,
            MacroField::Definition => MacroField::Name,
            MacroField::Args => MacroField::Definition,
            MacroField::Description => MacroField::Args,
            MacroField::Disabled => MacroField::Description,
            MacroField::IsEval => MacroField::Disabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_field_next_cycles() {
        assert_eq!(MacroField::Name.next(), MacroField::Definition);
        assert_eq!(MacroField::Definition.next(), MacroField::Args);
        assert_eq!(MacroField::Args.next(), MacroField::Description);
        assert_eq!(MacroField::Description.next(), MacroField::Disabled);
        assert_eq!(MacroField::Disabled.next(), MacroField::IsEval);
        assert_eq!(MacroField::IsEval.next(), MacroField::Name);
    }

    #[test]
    fn test_macro_field_previous_cycles() {
        assert_eq!(MacroField::Name.previous(), MacroField::IsEval);
        assert_eq!(MacroField::Definition.previous(), MacroField::Name);
        assert_eq!(MacroField::Args.previous(), MacroField::Definition);
        assert_eq!(MacroField::Description.previous(), MacroField::Args);
        assert_eq!(MacroField::Disabled.previous(), MacroField::Description);
        assert_eq!(MacroField::IsEval.previous(), MacroField::Disabled);
    }
}
