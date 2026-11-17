use crate::json::lexer::Token;
use crate::json::parser::Value;
use anyhow::Result;
use crate::json::parser::ascii_to_f64::ascii_to_f64;
use crate::json::parser::ascii_to_i64::ascii_to_i64;

#[derive(Clone)]
pub enum EntryKey {
    Null,
    Content(String)
}

impl EntryKey {
    pub fn get_content(self) -> Result<String> {
        if let EntryKey::Content(key) = self {
            Ok(key)
        } else {
            Err(anyhow::anyhow!("Entry key does not have a content value."))
        }
    }
}

impl Default for EntryKey {
    fn default() -> Self {
        EntryKey::Null
    }
}

pub struct ObjectParserState {
    current_entry_key: EntryKey,
    is_nested: bool,
    is_empty: bool,
    state: ObjectParserInternalState,
}

#[derive(Copy, Clone, Debug)]
pub enum ObjectParserInternalState {
    EntryKey,
    EntryColon,
    EntryValue,
    EntryCompleted,
    ObjectCompleted,
}

pub enum ParsingObjectTokenDecision {
    Continue,
    AddEntryWithScalarValue(EntryKey, Value),
    ObjectCompleted,
    NestedObjectFound,
    NestedArrayFound,
    StopParsing,
}

impl ObjectParserState {
    pub fn new() -> Self {
        ObjectParserState {
            current_entry_key: EntryKey::Null,
            is_nested: false,
            is_empty: true,
            state: ObjectParserInternalState::EntryKey,
        }
    }

    pub fn new_nested() -> Self {
        ObjectParserState {
            current_entry_key: EntryKey::Null,
            is_nested: true,
            is_empty: true,
            state: ObjectParserInternalState::EntryKey,
        }
    }

    pub fn is_nested(&self) -> bool {
        self.is_nested
    }

    // Used on keys with nested objects or arrays.
    pub fn current_entry_key(&mut self) -> EntryKey {
        std::mem::take(&mut self.current_entry_key)
    }

    pub fn next(&mut self, current_token: Token) -> Result<ParsingObjectTokenDecision> {
        match self.state {
            ObjectParserInternalState::EntryKey => {
                // Next token should be a colon.
                self.state = ObjectParserInternalState::EntryColon;

                match current_token {
                    Token::Text(text) => {
                        self.current_entry_key = EntryKey::Content(text);
                        self.is_empty = false;
                        Ok(ParsingObjectTokenDecision::Continue)
                    }
                    Token::ObjectEnd => {
                        if self.is_empty {
                            self.state = ObjectParserInternalState::ObjectCompleted;
                            Ok(ParsingObjectTokenDecision::ObjectCompleted)
                        } else {
                            Err(anyhow::anyhow!("Unexpected end of object."))
                        }
                    }
                    _ => {
                        Err(anyhow::anyhow!("The entry key should be a string"))
                    }
                }
            },
            ObjectParserInternalState::EntryColon => {
                if current_token != Token::Colon {
                    return Err(anyhow::anyhow!("Malformed object entry, expecting colon (:)"));
                }

                self.state = ObjectParserInternalState::EntryValue;
                Ok(ParsingObjectTokenDecision::Continue)
            },
            ObjectParserInternalState::EntryValue => {
                match current_token {
                    Token::Text(text) => {
                        self.state = ObjectParserInternalState::EntryCompleted;
                        // mem::take move ownership of value and replaces field to default value.
                        Ok(ParsingObjectTokenDecision::AddEntryWithScalarValue(
                            std::mem::take(&mut self.current_entry_key),
                            Value::String(text)
                        ))
                    }
                    Token::Integer(integer_string) => {
                        let integer = ascii_to_i64(integer_string);

                        self.state = ObjectParserInternalState::EntryCompleted;
                        // mem::take move ownership of value and replaces field to default value.
                        Ok(ParsingObjectTokenDecision::AddEntryWithScalarValue(
                            std::mem::take(&mut self.current_entry_key),
                            Value::Integer(integer)
                        ))
                    }
                    Token::Double(double_str) => {
                        let double = ascii_to_f64(double_str);

                        self.state = ObjectParserInternalState::EntryCompleted;
                        // mem::take move ownership of value and replaces field to default value.
                        Ok(ParsingObjectTokenDecision::AddEntryWithScalarValue(
                            std::mem::take(&mut self.current_entry_key),
                            Value::Double(double)
                        ))
                    }
                    Token::Boolean(boolean) => {
                        self.state = ObjectParserInternalState::EntryCompleted;
                        // mem::take move ownership of value and replaces field to default value.
                        Ok(ParsingObjectTokenDecision::AddEntryWithScalarValue(
                            std::mem::take(&mut self.current_entry_key),
                            Value::Boolean(boolean)
                        ))
                    }
                    Token::Null => {
                        self.state = ObjectParserInternalState::EntryCompleted;
                        // mem::take move ownership of value and replaces field to default value.
                        Ok(ParsingObjectTokenDecision::AddEntryWithScalarValue(
                            std::mem::take(&mut self.current_entry_key),
                            Value::Null
                        ))
                    }
                    Token::ObjectStart => {
                        // Next token is expected to be an entry complete
                        self.state = ObjectParserInternalState::EntryCompleted;
                        Ok(ParsingObjectTokenDecision::NestedObjectFound)
                    }
                    Token::ArrayStart => {
                        // Next token is expected to be an entry complete
                        self.state = ObjectParserInternalState::EntryCompleted;
                        Ok(ParsingObjectTokenDecision::NestedArrayFound)
                    }
                    _ => {
                        Err(anyhow::anyhow!("Malformed object entry, unexpected value"))
                    }
                }
            }
            ObjectParserInternalState::EntryCompleted => {
                // We can only expect, either a comma or and object closing token.
                match current_token {
                    Token::Comma => {
                        // Then we should parse a new entry
                        self.state = ObjectParserInternalState::EntryKey;
                        Ok(ParsingObjectTokenDecision::Continue)
                    }
                    Token::ObjectEnd => {
                        self.state = ObjectParserInternalState::ObjectCompleted;
                        Ok(ParsingObjectTokenDecision::ObjectCompleted)
                    }
                    _ => {
                        Err(anyhow::anyhow!("Malformed object expecting a , or }}"))
                    }
                }
            }
            ObjectParserInternalState::ObjectCompleted => {
                if current_token != Token::EndOfFile {
                    return Err(anyhow::anyhow!("Malformed object completed, expecting end of file"));
                }

                Ok(ParsingObjectTokenDecision::StopParsing)
            }
        }
    }
}