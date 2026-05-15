use anyhow::{Context, Result};
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

/// These are the possible tokens emmited by the lexer,
/// The parser consumes them, note tokens should not be used after consumed.
#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    ArrayStart,
    ArrayEnd,
    ObjectStart,
    ObjectEnd,
    Text(String),
    Comma,
    Colon,
    Boolean(bool),
    Integer(String),
    Float(String),
    Error(String),
    EndOfFile,
}

#[derive(Clone, Copy)]
pub enum AccumulableTokensKind {
    Text,
    Integer,
    Float,
}

#[derive(PartialEq, Eq)]
enum LexerStates {
    /// Looks for all possible delimiter tokens, these are all single byte tokens:
    /// eg: [, ], {, }, comma, colon, quote, t, f.
    ///
    /// Under the modern standards (RFC 8329 nad ECMA-404) a JSON is defined
    /// as any serialized JSON value. This mean you do not need to wrap your data in
    /// an object or an array for it to be considered valid.
    LookingDelimiters,

    /// Takes anythiing until next quote. Also looks for backslash quote (\")
    /// if is an esaped quote do not treat it as the end of text.
    LookingText,

    /// Look for all number cahracters until found something that is not
    /// a number character, eg spaces, commas, etc.
    ///
    /// If found a dot, do not end.
    ///
    /// If after dot is not a number, return error.
    ///
    /// If after dot have a number tracks everything until next non number
    /// character.
    LookingIntegerOrFloat,

    /// This state finish the iterator before reading the whole file,
    /// use this to finish iterator after finding an error.
    EarlyFinish,
}

/// A lexer receives a buffer reader and emits Tokens, which then are consumed
/// by the parser.
pub struct BufferedJsonLexer {
    buf_reader: BufReader<File>,
    state: LexerStates,

    /// Since we are reading from a buffer, we cannot keep a reference
    /// to original data, as is it will be later overwriten.
    current_token_accumulator: Vec<u8>,
    current_token_accumulator_kind: Option<AccumulableTokensKind>,
    is_current_token_float: bool,
}

impl BufferedJsonLexer {
    pub fn from_file(file_path: &str) -> Result<BufferedJsonLexer> {
        let f = File::open(file_path)
            .context("could not open the file when creating the BufferedJsonLexer")?;

        Ok(BufferedJsonLexer {
            // By default has a buffer capacity of 8 KiB
            buf_reader: BufReader::new(f),
            state: LexerStates::LookingDelimiters,
            current_token_accumulator: Vec::with_capacity(1024),
            current_token_accumulator_kind: None,
            is_current_token_float: false,
        })
    }
}

impl Iterator for BufferedJsonLexer {
    /// Returns a batch of tokens after reading from the file.
    type Item = Vec<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state == LexerStates::EarlyFinish {
            return None;
        }

        let data_chunk = self.buf_reader.fill_buf().ok()?;
        let data_chunk_len = data_chunk.len(); // TODO: Actually track this inside the for?
        let mut tokens_batch: Self::Item = Vec::new();

        if data_chunk_len == 0 {
            // If reached end of file, check if there are accumulated tokens to push.
            if let Some(token_kind) = self.current_token_accumulator_kind
                && self.current_token_accumulator.len() > 0
            {
                match token_kind {
                    AccumulableTokensKind::Float => {
                        if let Ok(numeric_value) =
                            String::from_utf8(self.current_token_accumulator.clone())
                        {
                            tokens_batch.push(Token::Float(numeric_value))
                        }
                    }
                    AccumulableTokensKind::Integer => {
                        if let Ok(numeric_value) =
                            String::from_utf8(self.current_token_accumulator.clone())
                        {
                            tokens_batch.push(Token::Integer(numeric_value))
                        }
                    }
                    AccumulableTokensKind::Text => {
                        if let Ok(text_value) =
                            String::from_utf8(self.current_token_accumulator.clone())
                        {
                            tokens_batch.push(Token::Text(text_value))
                        }
                    }
                }
            }

            tokens_batch.push(Token::EndOfFile);
            self.state = LexerStates::EarlyFinish; // Finish iterator on next iteration.
            // Finish iterator on end of file.
            return Some(tokens_batch);
        }

        // TODO: A multi byte token may be split among different reads of the buffer
        // we need to accumulate those bytes. For now we just worry about single byte tokens and ignore this case

        // We keept track here because we need to advance it from inside the state machine
        let mut i = 0;
        while i < data_chunk_len {
            let item = data_chunk[i];
            let mut delta = 1;

            match self.state {
                LexerStates::LookingDelimiters => {
                    match item {
                        b'[' => tokens_batch.push(Token::ArrayStart),
                        b']' => tokens_batch.push(Token::ArrayEnd),
                        b'{' => tokens_batch.push(Token::ObjectStart),
                        b'}' => tokens_batch.push(Token::ObjectEnd),
                        b',' => tokens_batch.push(Token::Comma),
                        b':' => tokens_batch.push(Token::Colon),
                        b'"' => self.state = LexerStates::LookingText,
                        b't' => {
                            // Checks if we have a true sequence.
                            if &data_chunk[i..i + 4] == b"true" {
                                tokens_batch.push(Token::Boolean(true));
                                delta = 4; // Advance after the "true" sequence
                            } else {
                                tokens_batch
                                    .push(Token::Error(String::from("found unexpected sequence")));
                                self.state = LexerStates::EarlyFinish;
                                break;
                            }
                        }
                        b'f' => {
                            // Checks if we have a false sequence.
                            if &data_chunk[i..i + 5] == b"false" {
                                tokens_batch.push(Token::Boolean(false));
                                delta = 5; // Advance after the "false" sequence.
                            } else {
                                tokens_batch
                                    .push(Token::Error(String::from("found unexpected sequence")));
                                self.state = LexerStates::EarlyFinish;
                                break;
                            }
                        }
                        b'0'..=b'9' | b'-' => {
                            // A json number cannot start with + symbol
                            self.state = LexerStates::LookingIntegerOrFloat;
                            // Intentionally do not move
                            continue;
                        }
                        b'\r' | b'\n' | b'\t' | b' ' => {} // Ignore these.
                        _ => {
                            if let Ok(item_str) = str::from_utf8(&[data_chunk[i]]) {
                                tokens_batch.push(Token::Error(format!(
                                    "unexpected character in JSON ({})",
                                    item_str
                                )));
                            } else {
                                tokens_batch.push(Token::Error(String::from(
                                    "unexpected character in JSON not a valid utf-8 encoded string",
                                )));
                            }

                            self.state = LexerStates::EarlyFinish;
                            break;
                        }
                    }
                }
                LexerStates::LookingText => {
                    if self.current_token_accumulator.len() + 1
                        > self.current_token_accumulator.capacity()
                    {
                        tokens_batch.push(Token::Error(String::from(
                            "a text cannot be larger than 1KiB",
                        )));
                        self.state = LexerStates::EarlyFinish;
                        break; // Stops reading bytes from current chunk as we found an error.
                    }

                    let prev_item = self.current_token_accumulator.last().unwrap_or_else(|| &0);

                    // Do not treat the quote as end of text if is preceded by a backslash.
                    if *prev_item != b'\\' && item == b'"' {
                        // TODO: Improve memory copies?
                        let text_bytes = self.current_token_accumulator.clone();
                        if let Ok(text_content) = String::from_utf8(text_bytes) {
                            tokens_batch.push(Token::Text(text_content));
                        } else {
                            tokens_batch
                                .push(Token::Error(String::from("the text is invalid utf8")));
                            self.state = LexerStates::EarlyFinish;
                            break;
                        }
                        self.current_token_accumulator.clear();
                        // Note we can have an empty string as key or value.
                        // End of string token, go back to main state.
                        self.state = LexerStates::LookingDelimiters;
                    } else {
                        // Accumulate, bytes for the text
                        self.current_token_accumulator.push(item);
                    }
                }
                LexerStates::LookingIntegerOrFloat => {
                    // A number cannot start with a leading 0, eg: 012 is invalid
                    // only 0. is valid
                    if let Some(first_item) = self.current_token_accumulator.get(0) {
                        if *first_item == b'-' && item == b'-' {
                            tokens_batch.push(Token::Error(String::from(
                                "a number cannot have multiple minus symbols",
                            )));
                            self.state = LexerStates::EarlyFinish;
                            break;
                        }

                        if *first_item == b'0'
                            && item != b'.'
                            && !self.is_current_token_float
                            && item >= b'0'
                            && item <= b'9'
                        {
                            tokens_batch.push(Token::Error(String::from(
                                "a number cannot have a leading 0",
                            )));
                            self.state = LexerStates::EarlyFinish;
                            break;
                        }
                    }

                    if item == b'.' {
                        if !self.is_current_token_float {
                            self.is_current_token_float = true;
                            self.current_token_accumulator.push(item);
                            i = i + delta; // Move to next character
                            continue;
                        } else {
                            tokens_batch.push(Token::Error(String::from(
                                "a float number can only have a single decimal separator",
                            )));
                            self.state = LexerStates::EarlyFinish;
                            break;
                        }
                    }

                    if self.is_current_token_float {
                        self.current_token_accumulator_kind = Some(AccumulableTokensKind::Float);
                    } else {
                        self.current_token_accumulator_kind = Some(AccumulableTokensKind::Integer);
                    }

                    // Finish number token if we found anything that is not a number or a dot or a minus
                    if (item < b'0' || item > b'9') && item != b'-' && item != b'.' {
                        if let Ok(numeric_value) =
                            String::from_utf8(self.current_token_accumulator.clone())
                        {
                            if self.is_current_token_float {
                                tokens_batch.push(Token::Float(numeric_value));
                            } else {
                                tokens_batch.push(Token::Integer(numeric_value));
                            }

                            self.current_token_accumulator.clear();
                            self.is_current_token_float = false;
                            self.current_token_accumulator_kind = None;
                            self.state = LexerStates::LookingDelimiters;
                            // Intentionally do not advance counter so that LookingDelimiters
                            // can analyze the current token

                            // Go to next item
                            continue;
                        } else {
                            tokens_batch.push(Token::Error(String::from(
                                "numeric value is not utf-8 encoded",
                            )));
                            self.state = LexerStates::EarlyFinish;
                            break;
                        }
                    };

                    self.current_token_accumulator.push(item);
                }
                _ => (),
            }

            i = i + delta;
        }

        self.buf_reader.consume(data_chunk_len);
        Some(tokens_batch)
    }
}

#[cfg(test)]
mod tests {
    use crate::json_lexer::{BufferedJsonLexer, Token};
    use crate::temporary_test_file::TemporaryTestFile;
    use anyhow::{Result, anyhow};

    #[test]
    fn should_get_expected_tokens_for_obj() -> Result<()> {
        let content = r#"{}"#;
        let temp_file = TemporaryTestFile::new("should_get_expected_tokens_for_obj.json", content)?;

        let file_path = temp_file.full_file_path().unwrap();
        let lexer = BufferedJsonLexer::from_file(file_path)?;
        let mut received_token: Vec<Token> = Vec::new();

        for tokens_batch in lexer {
            for token in tokens_batch {
                received_token.push(token);
            }
        }

        if received_token.len() != 3 {
            panic!("should find 3 tokens but got {}", received_token.len());
        }

        let expected_tokens = [Token::ObjectStart, Token::ObjectEnd, Token::EndOfFile];
        for (i, token) in received_token.iter().enumerate() {
            if *token != expected_tokens[i] {
                panic!(
                    "token ({:?}) do not match expected token ({:?})",
                    *token, expected_tokens[i]
                );
            }
        }

        temp_file.cleanup()?;
        Ok(())
    }

    #[test]
    fn should_get_expected_tokens_for_arr_inside_object() -> Result<()> {
        let content = r#"{"x1": [1, 2.0, "-4"]}"#;
        let temp_file = TemporaryTestFile::new(
            "should_get_expected_tokens_for_arr_inside_object.json",
            content,
        )?;

        let file_path = temp_file.full_file_path().unwrap();
        let lexer = BufferedJsonLexer::from_file(file_path)?;
        let mut received_token: Vec<Token> = Vec::new();

        for tokens_batch in lexer {
            for token in tokens_batch {
                received_token.push(token);
            }
        }

        if received_token.len() != 12 {
            panic!("should find 12 tokens but got {}", received_token.len());
        }

        let expected_tokens = [
            Token::ObjectStart,
            Token::Text(String::from("x1")),
            Token::Colon,
            Token::ArrayStart,
            Token::Integer(String::from("1")),
            Token::Comma,
            Token::Float(String::from("2.0")),
            Token::Comma,
            Token::Text(String::from("-4")),
            Token::ArrayEnd,
            Token::ObjectEnd,
            Token::EndOfFile,
        ];
        for (i, token) in received_token.iter().enumerate() {
            if *token != expected_tokens[i] {
                panic!(
                    "token ({:?}) do not match expected token ({:?})",
                    *token, expected_tokens[i]
                );
            }
        }

        temp_file.cleanup()?;
        Ok(())
    }

    #[test]
    fn should_err_if_number_begins_with_zero() -> Result<()> {
        let content = r#"0123.123"#;
        let temp_file =
            TemporaryTestFile::new("should_err_if_number_begins_with_zero.json", content)?;

        let file_path = temp_file.full_file_path().unwrap();
        let lexer = BufferedJsonLexer::from_file(file_path)?;
        let mut received_token: Vec<Token> = Vec::new();

        for tokens_batch in lexer {
            for token in tokens_batch {
                received_token.push(token);
            }
        }

        if received_token.len() != 1 {
            panic!("should find 1 tokens but got {}", received_token.len());
        }

        let expected_tokens = [Token::Error(String::from(
            "a number cannot have a leading 0",
        ))];
        for (i, token) in received_token.iter().enumerate() {
            if *token != expected_tokens[i] {
                panic!(
                    "token ({:?}) do not match expected token ({:?})",
                    *token, expected_tokens[i]
                );
            }
        }

        temp_file.cleanup()?;
        Ok(())
    }

    #[test]
    fn should_err_if_number_has_double_minus() -> Result<()> {
        let content = r#"--123"#;
        let temp_file =
            TemporaryTestFile::new("should_err_if_number_has_double_minus.json", content)?;

        let file_path = temp_file.full_file_path().unwrap();
        let lexer = BufferedJsonLexer::from_file(file_path)?;
        let mut received_token: Vec<Token> = Vec::new();

        for tokens_batch in lexer {
            for token in tokens_batch {
                received_token.push(token);
            }
        }

        if received_token.len() != 1 {
            panic!("should find 1 tokens but got {}", received_token.len());
        }

        let expected_tokens = [Token::Error(String::from(
            "a number cannot have multiple minus symbols",
        ))];
        for (i, token) in received_token.iter().enumerate() {
            if *token != expected_tokens[i] {
                panic!(
                    "token ({:?}) do not match expected token ({:?})",
                    *token, expected_tokens[i]
                );
            }
        }

        temp_file.cleanup()?;
        Ok(())
    }

    #[test]
    fn should_err_if_float_number_has_double_dot() -> Result<()> {
        let content = r#"0..23"#;
        let temp_file =
            TemporaryTestFile::new("should_err_if_float_number_has_double_dot.json", content)?;

        let file_path = temp_file.full_file_path().unwrap();
        let lexer = BufferedJsonLexer::from_file(file_path)?;
        let mut received_token: Vec<Token> = Vec::new();

        for tokens_batch in lexer {
            for token in tokens_batch {
                received_token.push(token);
            }
        }

        if received_token.len() != 1 {
            panic!("should find 1 tokens but got {}", received_token.len());
        }

        let expected_tokens = [Token::Error(String::from(
            "a float number can only have a single decimal separator",
        ))];
        for (i, token) in received_token.iter().enumerate() {
            if *token != expected_tokens[i] {
                panic!(
                    "token ({:?}) do not match expected token ({:?})",
                    *token, expected_tokens[i]
                );
            }
        }

        temp_file.cleanup()?;
        Ok(())
    }

    #[test]
    fn should_tokenize_numbers_correctly() -> Result<()> {
        let content = r#"0.22343, 123.123, -42.34, 12, -20"#;
        let temp_file = TemporaryTestFile::new("should_tokenize_numbers_correctly.json", content)?;

        let file_path = temp_file.full_file_path().unwrap();
        let lexer = BufferedJsonLexer::from_file(file_path)?;

        let mut received_token: Vec<Token> = Vec::new();

        for tokens_batch in lexer {
            for token in tokens_batch {
                received_token.push(token);
            }
        }

        println!("{:?}", received_token);
        if received_token.len() != 10 {
            panic!("should find 10 tokens but got {}", received_token.len());
        }

        let expected_tokens = [
            Token::Float(String::from("0.22343")),
            Token::Comma,
            Token::Float(String::from("123.123")),
            Token::Comma,
            Token::Float(String::from("-42.34")),
            Token::Comma,
            Token::Integer(String::from("12")),
            Token::Comma,
            Token::Integer(String::from("-20")),
            Token::EndOfFile,
        ];
        for (i, token) in received_token.iter().enumerate() {
            if *token != expected_tokens[i] {
                panic!(
                    "token ({:?}) do not match expected token ({:?})",
                    *token, expected_tokens[i]
                );
            }
        }

        temp_file.cleanup()?;
        Ok(())
    }

    #[test]
    fn should_tokenize_text_and_numbers_correctly() -> Result<()> {
        let content = r#"0.22343, 123.123, "hola,", -42.34, 12, -20, " mundo""#;
        let temp_file =
            TemporaryTestFile::new("should_tokenize_text_and_numbers_correctly.json", content)?;

        let file_path = temp_file.full_file_path().unwrap();
        let lexer = BufferedJsonLexer::from_file(file_path)?;

        let mut received_token: Vec<Token> = Vec::new();

        for tokens_batch in lexer {
            for token in tokens_batch {
                received_token.push(token);
            }
        }

        println!("{:?}", received_token);

        if received_token.len() != 14 {
            return Err(anyhow!("should find 14 tokens"));
        }

        let expected_tokens = [
            Token::Float(String::from("0.22343")),
            Token::Comma,
            Token::Float(String::from("123.123")),
            Token::Comma,
            Token::Text(String::from("hola,")),
            Token::Comma,
            Token::Float(String::from("-42.34")),
            Token::Comma,
            Token::Integer(String::from("12")),
            Token::Comma,
            Token::Integer(String::from("-20")),
            Token::Comma,
            Token::Text(String::from(" mundo")),
            Token::EndOfFile,
        ];
        for (i, token) in received_token.iter().enumerate() {
            if *token != expected_tokens[i] {
                panic!(
                    "token ({:?}) do not match expected token ({:?})",
                    *token, expected_tokens[i]
                );
            }
        }

        temp_file.cleanup()?;
        Ok(())
    }
}
