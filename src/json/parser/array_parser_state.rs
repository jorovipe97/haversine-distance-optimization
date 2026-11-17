use super::ascii_to_i64;
use crate::json::{lexer::Token, parser::Value};
use anyhow::{Result, anyhow};
use crate::json::parser::ascii_to_f64::ascii_to_f64;

#[derive(Debug)]
enum ArrayParserInternalState {
    ItemValue,
    ItemValueCompleted,
    ArrayCompleted,
}

/// This is responsible for parsing array items inside an array
/// It assumes tokens are inside an array.
///
/// It does not handles nested arrays as that is handled by the parser.
pub struct ArrayParserState {
    is_empty: bool,
    is_nested: bool,
    state: ArrayParserInternalState
}

pub enum ParsingArrayTokenDecision {
    AddItem(Value),
    Continue,
    NestedArrayFound,
    NestedObjectFound,
    ArrayCompleted,
    StopParsing,
}

impl ArrayParserState {
    pub fn new() -> ArrayParserState {
        ArrayParserState {
            is_empty: true,
            is_nested: false,
            state: ArrayParserInternalState::ItemValue
        }
    }

    pub fn new_nested() -> ArrayParserState {
        ArrayParserState {
            is_empty: true,
            is_nested: true,
            state: ArrayParserInternalState::ItemValue
        }
    }

    pub fn is_nested(&self) -> bool {
        self.is_nested
    }

    pub fn next(&mut self, current_token: Token) -> Result<ParsingArrayTokenDecision> {
        match self.state {
            ArrayParserInternalState::ItemValue => {
                match current_token {
                    Token::Text(text) => {
                        self.is_empty = false;
                        self.state = ArrayParserInternalState::ItemValueCompleted;
                        Ok(ParsingArrayTokenDecision::AddItem(Value::String(text)))
                    }
                    Token::Integer(integer_string) => {
                        self.is_empty = false;
                        let integer = ascii_to_i64(integer_string);
                        self.state = ArrayParserInternalState::ItemValueCompleted;
                        Ok(ParsingArrayTokenDecision::AddItem(Value::Integer(integer)))
                    }
                    Token::Double(double_str) => {
                        self.is_empty = false;
                        let double = ascii_to_f64(double_str);
                        self.state = ArrayParserInternalState::ItemValueCompleted;
                        Ok(ParsingArrayTokenDecision::AddItem(Value::Double(double)))
                    }
                    Token::Boolean(boolean) => {
                        self.is_empty = false;
                        self.state = ArrayParserInternalState::ItemValueCompleted;
                        Ok(ParsingArrayTokenDecision::AddItem(Value::Boolean(boolean)))
                    }
                    Token::Null => {
                        self.is_empty = false;
                        self.state = ArrayParserInternalState::ItemValueCompleted;
                        Ok(ParsingArrayTokenDecision::AddItem(Value::Null))
                    }
                    Token::ArrayEnd => {
                        if self.is_empty {
                            self.state = ArrayParserInternalState::ArrayCompleted;
                            return Ok(ParsingArrayTokenDecision::ArrayCompleted);
                        }

                        Err(anyhow!("Malformed array completed"))
                    }
                    Token::ObjectStart => {
                        // Next token is expected to be an entry complete
                        self.state = ArrayParserInternalState::ItemValueCompleted;
                        Ok(ParsingArrayTokenDecision::NestedObjectFound)
                    }
                    Token::ArrayStart => {
                        // Next token is expected to be an entry complete
                        self.state = ArrayParserInternalState::ItemValueCompleted;
                        Ok(ParsingArrayTokenDecision::NestedArrayFound)
                    }
                    _ => {
                        Err(anyhow!("Malformed array, unexpected value"))
                    }
                }
            },
            ArrayParserInternalState::ItemValueCompleted => {
                // Either a comma or an end of array
                match current_token {
                    Token::Comma => {
                        self.state = ArrayParserInternalState::ItemValue;
                        Ok(ParsingArrayTokenDecision::Continue)
                    }
                    Token::ArrayEnd => {
                        self.state = ArrayParserInternalState::ArrayCompleted;
                        Ok(ParsingArrayTokenDecision::ArrayCompleted)
                    }
                    _ => {
                        Err(anyhow!("Malformed array, unexpected value"))
                    }
                }
            }
            ArrayParserInternalState::ArrayCompleted => {
                if current_token != Token::EndOfFile {
                    return Err(anyhow!("Malformed array, expected end of file"));
                }

                Ok(ParsingArrayTokenDecision::StopParsing)
            }
        }
    }
}
