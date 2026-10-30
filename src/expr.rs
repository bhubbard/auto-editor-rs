use crate::audio::AudioThreshold;
use crate::motion::MotionThreshold;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditMethod {
    Audio {
        threshold: AudioThreshold,
        stream: usize,
    },
    Motion {
        threshold: MotionThreshold,
        stream: usize,
    },
    Const(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditExpr {
    Method(EditMethod),
    And(Vec<EditExpr>),
    Or(Vec<EditExpr>),
    Not(Box<EditExpr>),
}

impl EditExpr {
    pub fn parse(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed == "1" || trimmed.eq_ignore_ascii_case("all") || trimmed.eq_ignore_ascii_case("true") {
            return Ok(EditExpr::Method(EditMethod::Const(true)));
        }
        if trimmed == "0" || trimmed.eq_ignore_ascii_case("none") || trimmed.eq_ignore_ascii_case("false") {
            return Ok(EditExpr::Method(EditMethod::Const(false)));
        }

        let tokens = tokenize(trimmed)?;
        let mut cursor = 0;
        let expr = parse_expression(&tokens, &mut cursor)?;
        if cursor < tokens.len() {
            return Err(format!("Unexpected token after expression: {:?}", tokens[cursor]));
        }
        Ok(expr)
    }

    /// Evaluate expression at a specific frame given audio levels and motion level
    pub fn evaluate(&self, audio_levels: &[f64], motion_level: f64) -> bool {
        match self {
            EditExpr::Method(m) => match m {
                EditMethod::Const(b) => *b,
                EditMethod::Audio { threshold, stream } => {
                    let level = audio_levels.get(*stream).copied().unwrap_or(0.0);
                    level >= threshold.linear
                }
                EditMethod::Motion { threshold, .. } => motion_level >= threshold.threshold,
            },
            EditExpr::And(exprs) => exprs.iter().all(|e| e.evaluate(audio_levels, motion_level)),
            EditExpr::Or(exprs) => exprs.iter().any(|e| e.evaluate(audio_levels, motion_level)),
            EditExpr::Not(inner) => !inner.evaluate(audio_levels, motion_level),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    LParen,
    RParen,
    Identifier(String),
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '(' {
            tokens.push(Token::LParen);
            chars.next();
        } else if c == ')' {
            tokens.push(Token::RParen);
            chars.next();
        } else {
            let mut word = String::new();
            while let Some(&w) = chars.peek() {
                if w.is_whitespace() || w == '(' || w == ')' {
                    break;
                }
                word.push(w);
                chars.next();
            }
            tokens.push(Token::Identifier(word));
        }
    }
    Ok(tokens)
}

fn parse_expression(tokens: &[Token], cursor: &mut usize) -> Result<EditExpr, String> {
    if *cursor >= tokens.len() {
        return Err("Unexpected end of expression".to_string());
    }

    match &tokens[*cursor] {
        Token::LParen => {
            *cursor += 1; // consume '('
            if *cursor >= tokens.len() {
                return Err("Expected operator after '('".to_string());
            }

            let op_name = match &tokens[*cursor] {
                Token::Identifier(id) => id.to_lowercase(),
                other => return Err(format!("Expected operator name, found {:?}", other)),
            };
            *cursor += 1;

            let mut operands = Vec::new();
            while *cursor < tokens.len() && tokens[*cursor] != Token::RParen {
                let operand = parse_expression(tokens, cursor)?;
                operands.push(operand);
            }

            if *cursor >= tokens.len() || tokens[*cursor] != Token::RParen {
                return Err("Missing closing ')' in expression".to_string());
            }
            *cursor += 1; // consume ')'

            match op_name.as_str() {
                "or" => Ok(EditExpr::Or(operands)),
                "and" => Ok(EditExpr::And(operands)),
                "not" => {
                    if operands.len() != 1 {
                        return Err(format!("'not' operator expects exactly 1 operand, got {}", operands.len()));
                    }
                    Ok(EditExpr::Not(Box::new(operands.into_iter().next().unwrap())))
                }
                unknown => Err(format!("Unknown operator in expression: '{}'", unknown)),
            }
        }
        Token::Identifier(id) => {
            *cursor += 1;
            parse_single_method(id)
        }
        Token::RParen => Err("Unexpected closing ')'".to_string()),
    }
}

fn parse_single_method(id: &str) -> Result<EditExpr, String> {
    if id == "1" || id.eq_ignore_ascii_case("all") {
        return Ok(EditExpr::Method(EditMethod::Const(true)));
    }
    if id == "0" || id.eq_ignore_ascii_case("none") {
        return Ok(EditExpr::Method(EditMethod::Const(false)));
    }

    let parts: Vec<&str> = id.splitn(2, ':').collect();
    let method_name = parts[0].to_lowercase();
    let args = if parts.len() > 1 { parts[1] } else { "" };

    match method_name.as_str() {
        "audio" => {
            let mut threshold = AudioThreshold::default();
            let mut stream = 0;

            if !args.is_empty() {
                for pair in args.split(',') {
                    if let Some((k, v)) = pair.split_once('=') {
                        match k.trim().to_lowercase().as_str() {
                            "threshold" => threshold = AudioThreshold::parse(v)?,
                            "stream" => stream = if v.eq_ignore_ascii_case("all") { 0 } else { v.trim().parse().map_err(|e| format!("Invalid stream index: {}", e))? },
                            unknown => return Err(format!("Unknown audio parameter '{}'", unknown)),
                        }
                    } else {
                        // Positional first argument is threshold
                        threshold = AudioThreshold::parse(pair)?;
                    }
                }
            }
            Ok(EditExpr::Method(EditMethod::Audio { threshold, stream }))
        }
        "motion" => {
            let mut threshold = MotionThreshold::default();
            let stream = 0;

            if !args.is_empty() {
                for pair in args.split(',') {
                    if let Some((k, v)) = pair.split_once('=') {
                        match k.trim().to_lowercase().as_str() {
                            "threshold" => threshold = MotionThreshold::parse(v)?,
                            unknown => return Err(format!("Unknown motion parameter '{}'", unknown)),
                        }
                    } else {
                        threshold = MotionThreshold::parse(pair)?;
                    }
                }
            }
            Ok(EditExpr::Method(EditMethod::Motion { threshold, stream }))
        }
        unknown => Err(format!("Unknown edit method: '{}'", unknown)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let e = EditExpr::parse("audio:0.04").unwrap();
        assert_eq!(
            e,
            EditExpr::Method(EditMethod::Audio {
                threshold: AudioThreshold::from_linear(0.04),
                stream: 0
            })
        );
    }

    #[test]
    fn test_parse_boolean_or() {
        let e = EditExpr::parse("(or audio:-19dB motion:0.06)").unwrap();
        match e {
            EditExpr::Or(operands) => {
                assert_eq!(operands.len(), 2);
            }
            _ => panic!("Expected Or expression"),
        }
    }

    #[test]
    fn test_evaluation() {
        let e = EditExpr::parse("(or audio:0.1 motion:0.05)").unwrap();
        // Loud audio, quiet motion -> true
        assert!(e.evaluate(&[0.15], 0.01));
        // Quiet audio, high motion -> true
        assert!(e.evaluate(&[0.05], 0.08));
        // Quiet audio, quiet motion -> false
        assert!(!e.evaluate(&[0.05], 0.01));
    }
}
