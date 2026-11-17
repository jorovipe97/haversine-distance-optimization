use std::collections::HashMap;
use crate::json::lexer::Token;
use super::{JsonParser, Value};
use anyhow::Result;

fn parse(lexer_tokens: Vec<Token>) -> Result<Value> {
    let mut parser = JsonParser::new();
    parser.consume_tokens(lexer_tokens)?;
    parser.value()
}

#[test]
fn should_parse_root_scalars_jsons() -> Result<()> {
    // A valid json may contain just a single scalar, eg: a single number is a valid json according to spec.

    // Null
    let lexer_tokens = vec![Token::Null];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Null;
    assert_eq!(result, expected_result);

    // Integer
    let lexer_tokens = vec![Token::Integer(String::from("123"))];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Integer(123);
    assert_eq!(result, expected_result);

    // String
    let lexer_tokens = vec![Token::Text(String::from("hello"))];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::String(String::from("hello"));
    assert_eq!(result, expected_result);

    // Boolean
    let lexer_tokens = vec![Token::Boolean(true)];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Boolean(true);
    assert_eq!(result, expected_result);

    // Double
    let lexer_tokens = vec![Token::Double(String::from("123.45"))];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Double(123.45);
    assert_eq!(result, expected_result);

    Ok(())
}

#[test]
fn should_parse_when_root_scalar_is_followed_by_eof() -> Result<()> {
    let lexer_tokens = vec![Token::Null, Token::EndOfFile];
    let parse = parse(lexer_tokens);
    match parse {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow::anyhow!("parser should run without problem when root scalar is followed by eof token")),
    }
}

#[test]
fn should_error_when_root_scalar_is_followed_by_comma() -> Result<()> {
    let lexer_tokens = vec![Token::Null, Token::Comma];
    let parse = parse(lexer_tokens);
    match parse {
        Ok(_) => Err(anyhow::anyhow!("parser should fail when root scalar is followed by comma")),
        Err(_) => Ok(()),
    }
}

#[test]
fn should_error_when_input_is_just_and_eof() -> Result<()> {
    let lexer_tokens = vec![Token::EndOfFile];
    let parse = parse(lexer_tokens);
    match parse {
        Ok(_) => Err(anyhow::anyhow!("parser should fail when input is just an eof")),
        Err(_) => Ok(()),
    }
}

#[test]
fn should_parse_if_input_has_multiple_eof_tokens() -> Result<()> {
    let lexer_tokens = vec![Token::Null, Token::EndOfFile, Token::EndOfFile, Token::EndOfFile];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Null;

    assert_eq!(result, expected_result);

    Ok(())
}

#[test]
fn should_error_when_root_scalar_is_followed_by_array_start() -> Result<()> {
    let lexer_tokens = vec![Token::Null, Token::ArrayStart, Token::ArrayEnd];
    let parse = parse(lexer_tokens);
    match parse {
        Ok(_) => Err(anyhow::anyhow!("parser should fail when root scalar is followed by array")),
        Err(_) => Ok(()),
    }
}

#[test]
fn should_error_when_root_scalar_is_followed_by_colon() -> Result<()> {
    let lexer_tokens = vec![Token::Null, Token::Colon];
    let parse = parse(lexer_tokens);
    match parse {
        Ok(_) => Err(anyhow::anyhow!("parser should fail when root scalar is followed by colon")),
        Err(_) => Ok(()),
    }
}

#[test]
fn should_parse_nested_arrays() -> Result<()> {
    // [123, [234, [345]]]
    let lexer_tokens = vec![
        Token::ArrayStart, Token::Integer(String::from("123")), Token::Comma,
        Token::ArrayStart, Token::Integer(String::from("234")), Token::Comma,
        Token::ArrayStart, Token::Integer(String::from("345")), Token::ArrayEnd,
        Token::ArrayEnd, Token::ArrayEnd
    ];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Array(vec![
        Value::Integer(123),
        Value::Array(vec![
            Value::Integer(234),
            Value::Array(vec![
                Value::Integer(345),
            ])
        ])
    ]);

    assert_eq!(result, expected_result);

    Ok(())
}

#[test]
fn should_fail_when_array_only_has_comma() -> Result<()> {
    // [,]
    let lexer_tokens = vec![
        Token::ArrayStart, Token::Comma, Token::ArrayEnd
    ];

    let parse = parse(lexer_tokens);
    match parse {
        Ok(_) => Err(anyhow::anyhow!("parser should fail when array only has a comma")),
        Err(_) => Ok(()),
    }
}

#[test]
fn should_fail_when_array_has_trailing_comma() -> Result<()> {
    // [1,]
    let lexer_tokens = vec![
        Token::ArrayStart, Token::Integer(String::from("1")), Token::Comma, Token::ArrayEnd
    ];

    let parse = parse(lexer_tokens);
    match parse {
        Ok(_) => Err(anyhow::anyhow!("parser should fail when array has trailing comma")),
        Err(_) => Ok(()),
    }
}

#[test]
fn should_parse_array_with_multiple_types() -> Result<()> {
    let lexer_tokens = vec![
        Token::ArrayStart,
        Token::Integer(String::from("123")), Token::Comma,
        Token::Double(String::from("123.45")), Token::Comma,
        Token::Text(String::from("Hello World")), Token::Comma,
        Token::Boolean(true), Token::Comma,
        Token::Null,
        Token::ArrayEnd
    ];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Array(vec![
        Value::Integer(123),
        Value::Double(123.45),
        Value::String(String::from("Hello World")),
        Value::Boolean(true),
        Value::Null
    ]);

    assert_eq!(result, expected_result);

    Ok(())
}

#[test]
fn should_parse_nested_objects() -> Result<()> {
    // {
    //     "name": "jose",
    //     "age": 19,
    //     "address": {
    //         "city": "med",
    //         "country": "col",
    //         "coordinates": {
    //             "lat": 2.2,
    //             "lon": 5.4
    //          }
    //      }
    // }
    let lexer_tokens = vec![
        Token::ObjectStart, Token::Text(String::from("name")), Token::Colon, Token::Text(String::from("jose")),
        Token::Comma, Token::Text(String::from("age")), Token::Colon, Token::Integer(String::from("19")), Token::Comma, Token::Text(String::from("address")),
        Token::Colon, Token::ObjectStart, Token::Text(String::from("city")), Token::Colon, Token::Text(String::from("med")), Token::Comma, Token::Text(String::from("country")),
        Token::Colon, Token::Text(String::from("col")), Token::Comma, Token::Text(String::from("coordinates")), Token::Colon, Token::ObjectStart,
        Token::Text(String::from("lat")), Token::Colon, Token::Double(String::from("2.2")), Token::Comma, Token::Text(String::from("lon")), Token::Colon,
        Token::Double(String::from("5.4")), Token::ObjectEnd, Token::ObjectEnd, Token::ObjectEnd];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Object(HashMap::from([
        (String::from("name"), Value::String(String::from("jose"))),
        (String::from("age"), Value::Integer(19)),
        (String::from("address"), Value::Object(HashMap::from([
            (String::from("city"), Value::String(String::from("med"))),
            (String::from("country"), Value::String(String::from("col"))),
            (String::from("coordinates"), Value::Object(HashMap::from([
                (String::from("lat"), Value::Double(2.2)),
                (String::from("lon"), Value::Double(5.4)),
            ])))
        ]))),
    ]));

    assert_eq!(result, expected_result);

    Ok(())
}

#[test]
fn should_parse_object_with_scalars() -> Result<()> {
    // {
    //     "name": "jose",
    //     "age": 19,
    // }
    let lexer_tokens = vec![
        Token::ObjectStart, Token::Text(String::from("name")), Token::Colon, Token::Text(String::from("jose")),
        Token::Comma, Token::Text(String::from("age")), Token::Colon, Token::Integer(String::from("19")), Token::ObjectEnd];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Object(HashMap::from([
        (String::from("name"), Value::String(String::from("jose"))),
        (String::from("age"), Value::Integer(19)),
    ]));

    assert_eq!(result, expected_result);

    Ok(())
}

#[test]
fn should_parse_empty_object() -> Result<()> {
    // {}
    let lexer_tokens = vec![Token::ObjectStart, Token::ObjectEnd];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Object(HashMap::from([]));

    assert_eq!(result, expected_result);

    Ok(())
}

#[test]
fn should_parse_object_with_nested_array() -> Result<()> {
    // {"nums":[1,2]}
    let lexer_tokens = vec![
        Token::ObjectStart, Token::Text(String::from("nums")),
        Token::Colon, Token::ArrayStart, Token::Integer(String::from("1")),
        Token::Comma, Token::Integer(String::from("2")),
        Token::ArrayEnd, Token::ObjectEnd
    ];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Object(HashMap::from([
        (String::from("nums"), Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
        ]))
    ]));

    assert_eq!(result, expected_result);

    Ok(())
}

#[test]
fn should_parse_array_with_nested_object() -> Result<()> {
    // [{"name": "jose"}]
    let lexer_tokens = vec![
        Token::ArrayStart, Token::ObjectStart, Token::Text(String::from("name")),
        Token::Colon, Token::Text(String::from("jose")), Token::ObjectEnd, Token::ArrayEnd
    ];
    let result = parse(lexer_tokens)?;
    let expected_result = Value::Array(vec![
        Value::Object(HashMap::from([
            (String::from("name"), Value::String(String::from("jose"))),
        ]))
    ]);

    assert_eq!(result, expected_result);

    Ok(())
}