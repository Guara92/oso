use std::fmt;

use indoc::formatdoc;
use serde::{Deserialize, Serialize};

use super::{rules::Rule, sources::*, terms::*};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(into = "FormattedPolarError")]
pub struct PolarError {
    pub kind: ErrorKind,
    pub context: Option<ErrorContext>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct FormattedPolarError {
    pub kind: ErrorKind,
    pub formatted: String,
}

impl From<PolarError> for FormattedPolarError {
    fn from(other: PolarError) -> Self {
        Self {
            formatted: other.to_string(),
            kind: other.kind,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorKind {
    Parse(ParseError),
    Runtime(RuntimeError),
    Operational(OperationalError),
    Validation(ValidationError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub source: Source,
    pub row: usize,
    pub column: usize,
    pub include_location: bool,
}

impl PolarError {
    pub fn set_context(mut self, source: Option<&Source>, term: Option<&Term>) -> Self {
        if let (Some(source), Some(term)) = (source, term) {
            let (row, column) = crate::lexer::loc_to_pos(&source.src, term.offset());
            self.context.replace(ErrorContext {
                source: source.clone(),
                row,
                column,
                // @TODO(Sam): find a better way to include this info
                // TODO(gj|sam): this bool can probably be removed -- we should include
                // location unconditionally for errors that have the available context.
                include_location: matches!(
                    self.kind,
                    ErrorKind::Runtime(RuntimeError::UnhandledPartial { .. })
                ),
            });
        }
        self
    }
}

impl From<ParseError> for PolarError {
    fn from(err: ParseError) -> Self {
        Self {
            kind: ErrorKind::Parse(err),
            context: None,
        }
    }
}

impl From<RuntimeError> for PolarError {
    fn from(err: RuntimeError) -> Self {
        Self {
            kind: ErrorKind::Runtime(err),
            context: None,
        }
    }
}

impl From<OperationalError> for PolarError {
    fn from(err: OperationalError) -> Self {
        Self {
            kind: ErrorKind::Operational(err),
            context: None,
        }
    }
}

impl From<ValidationError> for PolarError {
    fn from(err: ValidationError) -> Self {
        Self {
            kind: ErrorKind::Validation(err),
            context: None,
        }
    }
}

pub type PolarResult<T> = std::result::Result<T, PolarError>;

impl std::error::Error for PolarError {}

impl fmt::Display for PolarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            ErrorKind::Parse(e) => write!(f, "{}", e)?,
            ErrorKind::Runtime(e) => write!(f, "{}", e)?,
            ErrorKind::Operational(e) => write!(f, "{}", e)?,
            ErrorKind::Validation(e) => write!(f, "{}", e)?,
        }
        if let Some(ref context) = self.context {
            write!(f, "{}", context)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParseError {
    IntegerOverflow {
        src_id: u64,
        token: String,
        loc: usize,
    },
    InvalidTokenCharacter {
        src_id: u64,
        token: String,
        c: char,
        loc: usize,
    },
    InvalidToken {
        src_id: u64,
        loc: usize,
    },
    #[allow(clippy::upper_case_acronyms)]
    UnrecognizedEOF {
        src_id: u64,
        loc: usize,
    },
    UnrecognizedToken {
        src_id: u64,
        token: String,
        loc: usize,
    },
    ExtraToken {
        src_id: u64,
        token: String,
        loc: usize,
    },
    ReservedWord {
        src_id: u64,
        token: String,
        loc: usize,
    },
    InvalidFloat {
        src_id: u64,
        token: String,
        loc: usize,
    },
    WrongValueType {
        src_id: u64,
        loc: usize,
        term: Term,
        expected: String,
    },
    DuplicateKey {
        src_id: u64,
        loc: usize,
        key: String,
    },
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // @TODO(Sam): find a better way to incorporate this info
        if self.include_location {
            writeln!(f, "found in:")?;
            write!(f, "{}", self.source.src.split('\n').nth(self.row).unwrap())?;
            write!(f, "\n{}^", " ".repeat(self.column))?;
        }
        write!(f, " at line {}, column {}", self.row + 1, self.column + 1)?;
        if let Some(ref filename) = self.source.filename {
            write!(f, " in file {}", filename)?;
        }
        Ok(())
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IntegerOverflow { token, .. } => {
                write!(f, "'{}' caused an integer overflow", token.escape_debug())
            }
            Self::InvalidTokenCharacter { token, c, .. } => write!(
                f,
                "'{}' is not a valid character. Found in {}",
                c.escape_debug(),
                token.escape_debug()
            ),
            Self::InvalidToken { .. } => write!(f, "found an unexpected sequence of characters"),
            Self::UnrecognizedEOF { .. } => write!(
                f,
                "hit the end of the file unexpectedly. Did you forget a semi-colon"
            ),
            Self::UnrecognizedToken { token, .. } => write!(
                f,
                "did not expect to find the token '{}'",
                token.escape_debug()
            ),
            Self::ExtraToken { token, .. } => write!(
                f,
                "did not expect to find the token '{}'",
                token.escape_debug()
            ),
            Self::ReservedWord { token, .. } => write!(
                f,
                "{} is a reserved Polar word and cannot be used here",
                token.escape_debug()
            ),
            Self::InvalidFloat { token, .. } => write!(
                f,
                "{} was parsed as a float, but is invalid",
                token.escape_debug()
            ),
            Self::WrongValueType { term, expected, .. } => {
                write!(f, "Wrong value type: {}. Expected a {}", term, expected)
            }
            Self::DuplicateKey { key, .. } => {
                write!(f, "Duplicate key: {}", key)
            }
        }
    }
}

// @TODO: Information about the context of the error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeError {
    ArithmeticError {
        msg: String,
    },
    Unsupported {
        msg: String,
    },
    TypeError {
        msg: String,
        stack_trace: Option<String>,
    },
    StackOverflow {
        limit: usize,
    },
    QueryTimeout {
        msg: String,
    },
    Application {
        msg: String,
        stack_trace: Option<String>,
    },
    FileLoading {
        msg: String,
    },
    IncompatibleBindings {
        msg: String,
    },
    UnhandledPartial {
        var: Symbol,
        term: Term,
    },
    DataFilteringFieldMissing {
        var_type: String,
        field: String,
    },
}

impl RuntimeError {
    pub fn add_stack_trace(&mut self, vm: &crate::vm::PolarVirtualMachine) {
        match self {
            Self::Application { stack_trace, .. } | Self::TypeError { stack_trace, .. } => {
                *stack_trace = Some(vm.stack_trace())
            }
            _ => {}
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ArithmeticError { msg } => write!(f, "Arithmetic error: {}", msg),
            Self::Unsupported { msg } => write!(f, "Not supported: {}", msg),
            Self::TypeError { msg, stack_trace } => {
                if let Some(stack_trace) = stack_trace {
                    writeln!(f, "{}", stack_trace)?;
                }
                write!(f, "Type error: {}", msg)
            }
            Self::StackOverflow { limit } => {
                write!(f, "Goal stack overflow! MAX_GOALS = {}", limit)
            }
            Self::QueryTimeout { msg } => write!(f, "Query timeout: {}", msg),
            Self::Application { msg, stack_trace } => {
                if let Some(stack_trace) = stack_trace {
                    writeln!(f, "{}", stack_trace)?;
                }
                write!(f, "Application error: {}", msg)
            }
            Self::FileLoading { msg } => write!(f, "Problem loading file: {}", msg),
            Self::IncompatibleBindings { msg } => {
                write!(f, "Attempted binding was incompatible: {}", msg)
            }
            Self::UnhandledPartial { var, term } => {
                write!(
                    f,
                    "Found an unhandled partial in the query result: {var}

This can happen when there is a variable used inside a rule
which is not related to any of the query inputs.

For example: f(_x) if y.a = 1 and y.b = 2;

In this example, the variable `y` is constrained by `a = 1 and b = 2`,
but we cannot resolve these constraints without further information.

The unhandled partial is for variable {var}.
The expression is: {expr}
",
                    var = var,
                    expr = term,
                )
            }
            Self::DataFilteringFieldMissing { var_type, field } => {
                let msg = formatdoc!(
                    r#"Unregistered field or relation: {var_type}.{field}

                    Please include `{field}` in the `fields` parameter of your
                    `register_class` call for {var_type}.  For example, in Python:

                        oso.register_class({var_type}, fields={{
                            "{field}": <type or relation>
                        }})

                    For more information please refer to our documentation:
                        https://docs.osohq.com/guides/data_filtering.html
                    "#,
                    var_type = var_type,
                    field = field
                );
                write!(f, "{}", msg)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationalError {
    Serialization {
        msg: String,
    },
    Unimplemented {
        msg: String,
    },
    /// Rust panics caught in the `polar-c-api` crate.
    Unknown,

    /// An invariant has been broken internally.
    InvalidState {
        msg: String,
    },
}

impl fmt::Display for OperationalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Serialization { msg } => write!(f, "Serialization error: {}", msg),
            Self::Unimplemented { msg } => write!(f, "{} is not yet implemented", msg),
            Self::InvalidState { msg } => write!(f, "Invalid state: {}", msg),
            Self::Unknown => write!(
                f,
                "We hit an unexpected error.\n\
                Please submit a bug report at <https://github.com/osohq/oso/issues>"
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    MissingRequiredRule {
        rule: Rule,
    },
    InvalidRule {
        /// Rule where the error arose, tracked for lexical context.
        rule: Rule,
        msg: String,
    },
    InvalidRuleType {
        /// Rule type where the error arose, tracked for lexical context.
        rule_type: Rule,
        msg: String,
    },
    UndefinedRuleCall {
        /// Term<Call> where the error arose, tracked for lexical context.
        term: Term,
    },
    ResourceBlock {
        /// Term where the error arose, tracked for lexical context.
        term: Term,
        msg: String,
        // TODO(gj): enum for RelatedInformation that has a variant for capturing "other relevant
        // terms" for a particular diagnostic, e.g., for a DuplicateResourceBlock error the
        // already-declared resource block would be relevant info for the error emitted on
        // redeclaration.
    },
    SingletonVariable {
        /// Term<Symbol> where the error arose, tracked for lexical context.
        term: Term,
    },
    UnregisteredClass {
        /// Term<Symbol> where the error arose, tracked for lexical context.
        term: Term,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidRule { rule, msg } => {
                write!(f, "Invalid rule: {} {}", rule, msg)
            }
            Self::InvalidRuleType { rule_type, msg } => {
                write!(f, "Invalid rule type: {}\n\t{}", rule_type, msg)
            }
            Self::UndefinedRuleCall { term } => {
                write!(f, "Call to undefined rule: {}", term)
            }
            Self::MissingRequiredRule { rule } => {
                write!(f, "Missing implementation for required rule {}", rule)
            }
            Self::ResourceBlock { msg, .. } => {
                write!(f, "{}", msg)
            }
            Self::SingletonVariable { term } => {
                write!(f, "Singleton variable {term} is unused or undefined; try renaming to _{term} or _", term=term)
            }
            Self::UnregisteredClass { term } => {
                write!(f, "Unregistered class: {}", term)
            }
        }
    }
}
