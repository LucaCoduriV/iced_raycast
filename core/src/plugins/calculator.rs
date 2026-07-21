//! Built-in calculator plugin.
//!
//! Surfaces a result whenever the query is an arithmetic expression. Serves as
//! the reference example of a query-driven [`Plugin`].

use std::sync::atomic::{AtomicU64, Ordering};

use super::{
    ActionEffect, Plugin, PluginAction, PluginMeta, PluginResult, Preference, PreferenceKind,
    PreferenceValue,
};

/// Option index of the "Decimal places" preference: 0 = Auto, else a fixed
/// count via [`DECIMAL_OPTIONS`].
const DECIMAL_OPTIONS: [&str; 4] = ["Auto", "2", "4", "6"];

/// Evaluates arithmetic queries into a copyable result.
pub struct Calculator {
    /// Selected index into [`DECIMAL_OPTIONS`] for the "Decimal places"
    /// preference. Interior-mutable so `set_preference` (which takes `&self`)
    /// can update it live.
    decimals_choice: AtomicU64,
}

impl Calculator {
    pub fn new() -> Self {
        Calculator {
            decimals_choice: AtomicU64::new(0),
        }
    }

    /// Format a result honoring the "Decimal places" preference: `Auto` trims to
    /// a sensible precision; a fixed choice shows exactly that many decimals.
    fn format_result(&self, value: f64) -> String {
        match self.decimals_choice.load(Ordering::Relaxed) {
            1 => format!("{value:.2}"),
            2 => format!("{value:.4}"),
            3 => format!("{value:.6}"),
            _ => format_number(value),
        }
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Calculator {
    fn id(&self) -> &str {
        "calculator"
    }

    fn metadata(&self) -> PluginMeta {
        PluginMeta {
            name: Some("Calculator".to_string()),
            author: Some("built-in".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            description: Some(
                "Do quick maths right from the search bar — arithmetic and unit-free \
                 expressions evaluate as you type, and the result copies with a keystroke."
                    .to_string(),
            ),
        }
    }

    fn preferences(&self) -> Vec<Preference> {
        vec![
            Preference {
                id: "copy_on_enter".to_string(),
                label: "Copy on Enter".to_string(),
                hint: "Pressing Enter copies the result to the clipboard.".to_string(),
                kind: PreferenceKind::Toggle(true),
            },
            Preference {
                id: "decimal_places".to_string(),
                label: "Decimal places".to_string(),
                hint: "How many digits to show after the point.".to_string(),
                kind: PreferenceKind::Select {
                    options: DECIMAL_OPTIONS.iter().map(|o| o.to_string()).collect(),
                    selected: self.decimals_choice.load(Ordering::Relaxed),
                },
            },
        ]
    }

    fn set_preference(&self, id: &str, value: PreferenceValue) {
        if id == "decimal_places"
            && let PreferenceValue::Choice(index) = value
        {
            self.decimals_choice.store(index, Ordering::Relaxed);
        }
    }

    fn query(&self, query: &str) -> Vec<PluginResult> {
        let expression = query.trim();

        // Only engage for expressions that actually look like math, so typing
        // an app name (or a bare number) doesn't surface a calculator row.
        if !looks_like_math(expression) {
            return Vec::new();
        }

        let Some(value) = evaluate(expression) else {
            return Vec::new();
        };

        let formatted = self.format_result(value);

        vec![PluginResult {
            source_id: self.id().to_string(),
            section: "Calculator".to_string(),
            title: formatted.clone(),
            subtitle: Some(normalize_expression(expression)),
            icon: None,
            glyph: Some('='),
            actions: vec![PluginAction {
                label: "Copy to Clipboard".to_string(),
                effect: ActionEffect::CopyToClipboard(formatted),
            }],
        }]
    }
}

/// True when `s` is composed only of arithmetic characters and contains at
/// least one binary operator (so `"5"` or `"firefox"` don't trigger).
fn looks_like_math(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let is_allowed = |c: char| c.is_ascii_digit() || " \t.+-*/%^()×÷·".contains(c);
    if !s.chars().all(is_allowed) {
        return false;
    }

    has_binary_operator(s)
}

/// Whether the expression contains an infix operator. A leading `-`/`+`
/// (unary sign) does not count, so `"-5"` is treated as a plain number.
fn has_binary_operator(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if "+*/%^×÷·".contains(c) {
            return true;
        }

        if c == '-' {
            let preceding = chars[..i].iter().rev().find(|p| !p.is_whitespace());
            if matches!(preceding, Some(&p) if p.is_ascii_digit() || p == ')' || p == '.') {
                return true;
            }
        }
    }

    false
}

/// Evaluate an arithmetic expression, or `None` if it doesn't parse or the
/// result isn't finite (e.g. division by zero).
pub fn evaluate(input: &str) -> Option<f64> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return None;
    }

    let mut parser = Parser { tokens, pos: 0 };
    let result = parser.parse()?;

    result.is_finite().then_some(result)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' => {
                chars.next();
            }
            '0'..='9' | '.' => {
                let mut number = String::new();
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_digit() || d == '.' {
                        number.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(number.parse().ok()?));
            }
            '+' => push(&mut tokens, &mut chars, Token::Plus),
            '-' => push(&mut tokens, &mut chars, Token::Minus),
            '*' | '×' | '·' => push(&mut tokens, &mut chars, Token::Star),
            '/' | '÷' => push(&mut tokens, &mut chars, Token::Slash),
            '%' => push(&mut tokens, &mut chars, Token::Percent),
            '^' => push(&mut tokens, &mut chars, Token::Caret),
            '(' => push(&mut tokens, &mut chars, Token::LParen),
            ')' => push(&mut tokens, &mut chars, Token::RParen),
            _ => return None,
        }
    }

    Some(tokens)
}

fn push(tokens: &mut Vec<Token>, chars: &mut std::iter::Peekable<std::str::Chars>, token: Token) {
    tokens.push(token);
    chars.next();
}

/// Recursive-descent parser. Grammar (lowest to highest precedence):
/// `expr = term (('+'|'-') term)*`, `term = power (('*'|'/'|'%') power)*`,
/// `power = unary ('^' power)?` (right-assoc), `unary = ('-'|'+') unary | primary`,
/// `primary = number | '(' expr ')'`.
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn parse(&mut self) -> Option<f64> {
        let value = self.expr()?;
        // Reject trailing garbage like "2 3" or "2)".
        (self.pos == self.tokens.len()).then_some(value)
    }

    fn expr(&mut self) -> Option<f64> {
        let mut value = self.term()?;
        while let Some(token) = self.peek() {
            match token {
                Token::Plus => {
                    self.advance();
                    value += self.term()?;
                }
                Token::Minus => {
                    self.advance();
                    value -= self.term()?;
                }
                _ => break,
            }
        }
        Some(value)
    }

    fn term(&mut self) -> Option<f64> {
        let mut value = self.power()?;
        while let Some(token) = self.peek() {
            match token {
                Token::Star => {
                    self.advance();
                    value *= self.power()?;
                }
                Token::Slash => {
                    self.advance();
                    value /= self.power()?;
                }
                Token::Percent => {
                    self.advance();
                    value %= self.power()?;
                }
                _ => break,
            }
        }
        Some(value)
    }

    fn power(&mut self) -> Option<f64> {
        let base = self.unary()?;
        if matches!(self.peek(), Some(Token::Caret)) {
            self.advance();
            let exponent = self.power()?; // right-associative
            Some(base.powf(exponent))
        } else {
            Some(base)
        }
    }

    fn unary(&mut self) -> Option<f64> {
        match self.peek() {
            Some(Token::Minus) => {
                self.advance();
                Some(-self.unary()?)
            }
            Some(Token::Plus) => {
                self.advance();
                self.unary()
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Option<f64> {
        match self.advance()? {
            Token::Number(n) => Some(n),
            Token::LParen => {
                let value = self.expr()?;
                matches!(self.advance()?, Token::RParen).then_some(value)
            }
            _ => None,
        }
    }
}

/// Format a result: integers without a decimal point, otherwise trimmed to a
/// sensible precision.
fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        let formatted = format!("{value:.10}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Present the entered expression with pretty operators for the subtitle.
fn normalize_expression(s: &str) -> String {
    s.replace('*', "×")
        .replace('/', "÷")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_arithmetic() {
        assert_eq!(evaluate("1024 * 8"), Some(8192.0));
        assert_eq!(evaluate("2 + 3 * 4"), Some(14.0));
        assert_eq!(evaluate("(2 + 3) * 4"), Some(20.0));
        assert_eq!(evaluate("2 ^ 10"), Some(1024.0));
        assert_eq!(evaluate("2 ^ 2 ^ 3"), Some(256.0)); // right-assoc: 2^(2^3)
        assert_eq!(evaluate("-5 + 3"), Some(-2.0));
        assert_eq!(evaluate("10 / 4"), Some(2.5));
        assert_eq!(evaluate("10 % 3"), Some(1.0));
        assert_eq!(evaluate("3.14 * 2"), Some(6.28));
    }

    #[test]
    fn rejects_non_expressions() {
        assert_eq!(evaluate("firefox"), None);
        assert_eq!(evaluate("2 +"), None);
        assert_eq!(evaluate("2 3"), None);
        assert_eq!(evaluate("(2 + 3"), None);
        assert_eq!(evaluate("1 / 0"), None); // not finite
    }

    #[test]
    fn math_heuristic() {
        assert!(looks_like_math("5 + 5"));
        assert!(looks_like_math("1024*8"));
        assert!(looks_like_math("(2+3)/4"));
        assert!(!looks_like_math("5")); // plain number, no operator
        assert!(!looks_like_math("-5")); // unary sign only
        assert!(!looks_like_math("firefox"));
        assert!(!looks_like_math(""));
    }

    #[test]
    fn produces_result() {
        let results = Calculator::new().query("1024 * 8");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "8192");
        assert_eq!(results[0].subtitle.as_deref(), Some("1024 × 8"));
        assert!(Calculator::new().query("firefox").is_empty());
    }

    #[test]
    fn decimal_places_preference_changes_formatting() {
        let calc = Calculator::new();
        // Default is Auto: integers show no decimals.
        assert_eq!(calc.query("1024 * 8")[0].title, "8192");

        // Choosing "2" (option index 1) shows two fixed decimals.
        calc.set_preference("decimal_places", PreferenceValue::Choice(1));
        assert_eq!(calc.query("1024 * 8")[0].title, "8192.00");
        assert_eq!(calc.query("10 / 4")[0].title, "2.50");

        // "6" (index 3) shows six.
        calc.set_preference("decimal_places", PreferenceValue::Choice(3));
        assert_eq!(calc.query("10 / 4")[0].title, "2.500000");

        // Back to Auto (index 0).
        calc.set_preference("decimal_places", PreferenceValue::Choice(0));
        assert_eq!(calc.query("10 / 4")[0].title, "2.5");
    }
}
