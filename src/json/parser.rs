mod array_parser_state;
mod ascii_to_f64;
mod ascii_to_i64;
mod object_parser_state;

#[cfg(test)]
mod parser_tests;

use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use array_parser_state::ArrayParserState;
use ascii_to_f64::ascii_to_f64;
use ascii_to_i64::ascii_to_i64;
use object_parser_state::ObjectParserState;

use crate::{
    json::{lexer::Token, parser::array_parser_state::ParsingArrayTokenDecision},
};
use crate::json::parser::object_parser_state::ParsingObjectTokenDecision;

#[derive(Debug)]
enum ParserStates {
    Initial,
    ParsingArray,
    ParsingObject,

    /// A nested value can be an array or an object inside other array or object.
    CompletedNestedValue,

    StopParsing,
}

// Any valid JSON data can be manipulated in the following recursive enum representation.
// This data structure is copied from serde_json::Value.
// https://github.com/serde-rs/json#operating-on-untyped-json-values
#[derive(Debug, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Double(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

pub enum StackItem {
    Scalar(Value),
    Array(Vec<Value>, ArrayParserState),
    Object(HashMap<String, Value>, ObjectParserState),
}

impl StackItem {
    fn bool(value: bool) -> StackItem {
        StackItem::Scalar(Value::Boolean(value))
    }

    fn string(value: String) -> StackItem {
        StackItem::Scalar(Value::String(value))
    }

    fn double(value: f64) -> StackItem {
        StackItem::Scalar(Value::Double(value))
    }

    fn integer(value: i64) -> StackItem {
        StackItem::Scalar(Value::Integer(value))
    }

    fn null() -> StackItem {
        StackItem::Scalar(Value::Null)
    }

    fn array() -> StackItem {
        // Initial capacity is a yolo value
        StackItem::Array(Vec::with_capacity(16), ArrayParserState::new())
    }

    /// Array inside other structure, either object or array.
    fn nested_array() -> StackItem {
        StackItem::Array(Vec::with_capacity(16), ArrayParserState::new_nested())
    }

    fn object() -> StackItem {
        // Initial capacity is a yolo value
        StackItem::Object(HashMap::with_capacity(16), ObjectParserState::new())
    }

    fn nested_object() -> StackItem {
        // Initial capacity is a yolo value
        StackItem::Object(HashMap::with_capacity(16), ObjectParserState::new_nested())
    }
}

pub struct JsonParser {
    output_stack: Vec<StackItem>,
    state: ParserStates,
}

impl JsonParser {
    pub fn new() -> JsonParser {
        JsonParser {
            // We preallocate the stack to have up to 8 nested values.
            output_stack: Vec::with_capacity(8),
            state: ParserStates::Initial,
        }
    }

    /// Consumes the tokens to parse the json. Tokens ownership now belongs
    /// to the parser.
    pub fn consume_tokens(&mut self, mut tokens: Vec<Token>) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            // Takes ownership of the token at index i, and replace it with
            // Token::Consumed in the original array.
            let token = std::mem::replace(&mut tokens[i], Token::Consumed);
            i = i + 1;

            // Check if token is an error
            if let Token::Error(val) = token {
                return Err(anyhow!("error while lexing: {}", val));
            }

            match self.state {
                ParserStates::Initial => {
                    match token {
                        Token::Boolean(val) => {
                            self.output_stack.push(StackItem::bool(val));
                            // If root is a scalar we do not expect anything else but an end of file token.
                            self.state = ParserStates::StopParsing;
                        }
                        Token::Text(val) => {
                            self.output_stack.push(StackItem::string(val));
                            // If root is a scalar we do not expect anything else but an end of file token.
                            self.state = ParserStates::StopParsing;
                        }
                        Token::Integer(val) => {
                            self.output_stack
                                .push(StackItem::integer(ascii_to_i64(val)));
                            // If root is a scalar we do not expect anything else but an end of file token.
                            self.state = ParserStates::StopParsing;
                        }
                        Token::Double(val) => {
                            self.output_stack.push(StackItem::double(ascii_to_f64(val)));
                            // If root is a scalar we do not expect anything else but an end of file token.
                            self.state = ParserStates::StopParsing;
                        }
                        Token::Null => {
                            self.output_stack.push(StackItem::null());
                            // If root is a scalar we do not expect anything else but an end of file token.
                            self.state = ParserStates::StopParsing;
                        }
                        Token::Comma => {
                            // On error we do not change any state as we assume parser will not continue
                            // consuming tokens.
                            return Err(anyhow!("json cannot start with a comma"));
                        }
                        Token::ArrayStart => {
                            self.output_stack.push(StackItem::array());
                            self.state = ParserStates::ParsingArray;
                        }
                        Token::ObjectStart => {
                            self.output_stack.push(StackItem::object());
                            self.state = ParserStates::ParsingObject;
                        }
                        Token::EndOfFile => {
                            // If input string is empty, is not a valid json.
                            // https://stackoverflow.com/a/30621997
                            return Err(anyhow!("json cannot start with end of file"));
                        }
                        _ => (),
                    }
                }
                ParserStates::ParsingArray => {
                    let top = self.stack_peek_mut()?;
                    if let StackItem::Array(array_data, array_parser_state) = top {
                        let decision = array_parser_state.next(token)?;
                        match decision {
                            ParsingArrayTokenDecision::Continue => {
                                continue;
                            }
                            ParsingArrayTokenDecision::AddItem(item_value) => {
                                array_data.push(item_value);
                                continue;
                            }
                            ParsingArrayTokenDecision::NestedArrayFound => {
                                self.output_stack.push(StackItem::nested_array());
                                continue
                            }
                            ParsingArrayTokenDecision::NestedObjectFound => {
                                self.output_stack.push(StackItem::nested_object());
                                self.state = ParserStates::ParsingObject;
                                continue;
                            }
                            ParsingArrayTokenDecision::ArrayCompleted => {
                                if array_parser_state.is_nested() {
                                    self.state = ParserStates::CompletedNestedValue;

                                    // Do not move to token after the closing } because
                                    // ParserStates::CompletedNestedValue do not use it.
                                    // After the nested value is added to the root value, it will consume
                                    // the following token.
                                    i = i - 1;
                                }
                                continue;
                            }
                            ParsingArrayTokenDecision::StopParsing => {
                                self.state = ParserStates::StopParsing;
                                continue;
                            }
                        }
                    }
                },
                ParserStates::ParsingObject => {
                    let top = self.stack_peek_mut()?;
                    if let StackItem::Object(map_data, object_parser_state) = top {
                        let decision = object_parser_state.next(token)?;
                        match decision {
                            ParsingObjectTokenDecision::Continue => {
                                continue;
                            },
                            ParsingObjectTokenDecision::AddEntryWithScalarValue(key, value) => {
                                // If and object has repeated keys, we overwrite prev values.
                                // Under the RFC 8259 spec, this is undefined behavior, is parser specific.
                                // in JS parser we have this behavior, so we are happy replicating it here.
                                let key_content = key.get_content()?;
                                map_data.insert(key_content, value);
                                continue;
                            },
                            ParsingObjectTokenDecision::NestedObjectFound => {
                                self.output_stack.push(StackItem::nested_object());
                                continue;
                            }
                            ParsingObjectTokenDecision::NestedArrayFound => {
                                self.output_stack.push(StackItem::nested_array());
                                self.state = ParserStates::ParsingArray;
                                continue;
                            }
                            ParsingObjectTokenDecision::ObjectCompleted => {
                                if object_parser_state.is_nested() {
                                    self.state = ParserStates::CompletedNestedValue;

                                    // Do not move to token after the closing } because
                                    // ParserStates::CompletedNestedValue do not use it.
                                    // After the nested value is added to the root value, it will consume
                                    // the following token.
                                    i = i - 1;
                                }
                                continue;
                            }
                            ParsingObjectTokenDecision::StopParsing => {
                                self.state = ParserStates::StopParsing;
                                continue;
                            }
                        }
                    }
                }
                ParserStates::CompletedNestedValue => {
                    let nested_value = self.stack_pop()?;
                    let root_value = self.stack_peek_mut()?;

                    match nested_value {
                        StackItem::Object(nested_map_data, _) => {
                            match root_value {
                                StackItem::Object(root_map_data, root_parser_state) => {
                                    let root_key = root_parser_state.current_entry_key();
                                    let root_key_content = root_key.get_content()?;
                                    root_map_data.insert(root_key_content, Value::Object(nested_map_data));

                                    // Continue parsing the root object.
                                    self.state = ParserStates::ParsingObject;
                                },
                                StackItem::Array(root_array_data, _) => {
                                    root_array_data.push(Value::Object(nested_map_data));
                                    // Continue parsing the root array
                                    self.state = ParserStates::ParsingArray;
                                }
                                _ => return Err(anyhow!("not supported")),
                            }
                        },
                        StackItem::Array(nested_array_data, _) => {
                            match root_value {
                                StackItem::Array(root_array_data, _) => {
                                    root_array_data.push(Value::Array(nested_array_data));

                                    // Continue parsing the root array
                                    self.state = ParserStates::ParsingArray;
                                }
                                StackItem::Object(root_map_data, root_parser_state) => {
                                    let root_key = root_parser_state.current_entry_key();
                                    let root_key_content = root_key.get_content()?;
                                    root_map_data.insert(root_key_content, Value::Array(nested_array_data));

                                    // Continue parsing the root map
                                    self.state = ParserStates::ParsingObject;
                                }
                                _ => return Err(anyhow!("not supported")),
                            }
                        }
                        _ => return Err(anyhow!("not supported")),
                    }
                }
                ParserStates::StopParsing => {
                    if token != Token::EndOfFile {
                        return Err(anyhow!("unexpected token after stop parsing"));
                    }

                    break;
                }
            }
        }
        Ok(())
    }

    fn stack_peek_mut(&mut self) -> Result<&mut StackItem> {
        let last_index = self.output_stack.len() - 1;
        let top = self
            .output_stack
            .get_mut(last_index)
            .context("output stack is empty")?;

        Ok(top)
    }

    fn stack_pop(&mut self) -> Result<StackItem> {
        let last = self.output_stack.pop().context("output stack is empty")?;
        Ok(last)
    }

    /// Returns the json value after parsing
    pub fn value(&mut self) -> Result<Value> {
        if self.output_stack.len() == 0 {
            return Err(anyhow!("invalid json nothing in the output stack"));
        }

        if self.output_stack.len() > 1 {
            return Err(anyhow!("invalid json nested values not consumed"));
        }

        let stack_root = self.output_stack.pop().context("cannot get root json value")?;
        match stack_root {
            StackItem::Scalar(val) => Ok(val),
            StackItem::Array(array_data, _) => {
                Ok(Value::Array(array_data))
            }
            StackItem::Object(object_data, _) => {
                Ok(Value::Object(object_data))
            }
        }
    }
}
