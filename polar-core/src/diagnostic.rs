use std::fmt;

use super::error::{ErrorContext, PolarError};
use super::kb::KnowledgeBase;

#[derive(Debug)]
pub enum Diagnostic {
    Error(PolarError),
    Warning(String),
}

impl Diagnostic {
    pub fn is_error(&self) -> bool {
        matches!(self, Diagnostic::Error(_))
    }

    pub fn is_parse_error(&self) -> bool {
        use super::error::ErrorKind::Parse;
        matches!(self, Diagnostic::Error(PolarError { kind: Parse(_), .. }))
    }

    // TODO(gj): ErrorContext -> generic DiagnosticContext type once we add structure to warnings.
    pub fn add_context(&mut self, context: ErrorContext) {
        match self {
            Diagnostic::Error(e) => e.context.replace(context),
            Diagnostic::Warning(_) => todo!(),
        };
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Diagnostic::Error(e) => write!(f, "{}", e)?,
            Diagnostic::Warning(w) => write!(f, "{}", w)?,
        }
        Ok(())
    }
}

// Attach context to diagnostics.
//
// TODO(gj): can we attach context to *all* errors here since all errors will be parse-time
// errors and so will have some source context to attach? NOTE(gj): not all -- some errors
// like the absence of an allow rule don't pertain to a particular file or location
// therein.
pub fn set_context_for_diagnostics(kb: &KnowledgeBase, diagnostics: &mut Vec<Diagnostic>) {
    use super::error::{ErrorKind::*, ParseError, ParseErrorKind::*, ValidationError::*};

    for diagnostic in diagnostics {
        let context = match diagnostic {
            Diagnostic::Error(e) => match e.kind {
                Parse(ParseError { ref kind, src_id }) => match kind {
                    DuplicateKey { key: token, loc }
                    | ExtraToken { token, loc }
                    | IntegerOverflow { token, loc }
                    | InvalidFloat { token, loc }
                    | ReservedWord { token, loc }
                    | UnrecognizedToken { token, loc } => Some(((*loc, loc + token.len()), src_id)),
                    InvalidTokenCharacter { loc, .. }
                    | InvalidToken { loc }
                    | UnrecognizedEOF { loc } => Some(((*loc, *loc), src_id)),
                    WrongValueType { term, .. } => term.span().map(|span| (span, src_id)),
                },

                Validation(ResourceBlock { ref term, .. })
                | Validation(SingletonVariable { ref term, .. })
                | Validation(UndefinedRule { ref term })
                | Validation(UnregisteredClass { ref term, .. }) => {
                    term.span().zip(term.get_source_id())
                }

                // TODO(gj): Track source for all three of these.
                Validation(InvalidRule { .. })
                | Validation(InvalidRuleType { .. })
                | Validation(MissingRequiredRule { .. }) => None,

                Runtime(_) | Operational(_) => None,
            },
            Diagnostic::Warning(_) => None,
        };
        if let Some(((left, _right), src_id)) = context {
            if let Some(source) = kb.sources.get_source(src_id) {
                let (row, column) = crate::lexer::loc_to_pos(&source.src, left);
                diagnostic.add_context(ErrorContext {
                    source,
                    row,
                    column,
                    include_location: false,
                })
            }
        }
    }
}
