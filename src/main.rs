use std::collections::HashMap;
use anyhow::{anyhow, Result};
use winnow::{Parser, PResult, seq};
use winnow::ascii::{digit1, multispace0};
use winnow::combinator::{alt, delimited, opt, separated, separated_pair};
use winnow::error::{ContextError, ErrMode};
use winnow::token::{one_of, take_until};

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

fn main() {
    let s = r#"{
        "name": "John Doe",
        "age": 30,
        "is_student": false,
        "marks": [90.0, -80.0, 85.1],
        "address": {
            "city": "New York",
            "zip": 10001
        },
        "nested": {
            "different_element_array": [1, null, true, "hello", { "a": 1, "s": "str" }],
            "empty_arr": [],
            "empty_obj": {}
        },
        "small_number": 0.00000000000005,
        "scientific_number": -1.1e-30,
        "scientific_number2": -1.1e+1
    }"#;

    let input = &mut (&*s);
    let v = parse_json(input);

    match v {
        Ok(json) => println!("Successfully parsed JSON: {:?}", json),
        Err(e) => println!("Failed to parse JSON: {:?}", e)
    }
}

fn parse_json(input: &mut &str) -> Result<JsonValue> {
    parse_value(input)
        .map_err(|e| anyhow!("Failed to parse JSON: {:?}", e))
}

fn parse_null(input: &mut &str) -> PResult<()> {
    "null".value(()).parse_next(input)
}

fn parse_string(input: &mut &str) -> PResult<String> {
    let ret = delimited('"', take_until(0.., '"') ,'"').parse_next(input)?;
    Ok(ret.to_string())
}

fn parse_number(input: &mut &str) -> PResult<f64> {
    let sign = opt("-").map(|x| x.is_some()).parse_next(input)?;
    let num = digit1.parse_to::<f64>().parse_next(input)?;
    let ret: Result<(), ErrMode<ContextError>> = ".".value(()).parse_next(input);

    if ret.is_ok() {
        let frac = digit1.parse_to::<String>().parse_next(input)?;
        let fraction_length = frac.to_string().len();
        let v = frac.parse::<f64>().unwrap();
        let fraction_value = v / 10_f64.powi(fraction_length as i32);

        let v = num + fraction_value;
        Ok(if sign { -v } else { v })
    } else {
        Ok(if sign { -num } else { num })
    }
}

fn parse_boolean(input: &mut &str) -> PResult<bool> {
   alt(("true", "false")).parse_to().parse_next(input)
}

fn parse_array(input: &mut &str) -> PResult<Vec<JsonValue>> {
    let comma_with_space = delimited(multispace0, ",", multispace0);
    let sep_left = delimited(multispace0, "[", multispace0);
    let sep_right = delimited(multispace0, "]", multispace0);

    let parse_values = separated(0.., parse_value, comma_with_space);

    let ret = delimited(sep_left, parse_values, sep_right).parse_next(input)?;

    Ok(ret)
}

fn parse_object(input: &mut &str) -> PResult<HashMap<String, JsonValue>> {
    let colon_with_space = delimited(multispace0, ":", multispace0);
    let comma_with_space = delimited(multispace0, ",", multispace0);
    let sep_left = delimited(multispace0, "{", multispace0);
    let sep_right = delimited(multispace0, "}", multispace0);

    let parse_kv_pair = separated_pair(parse_string, colon_with_space, parse_value);
    let parse_kv = separated(0.., parse_kv_pair, comma_with_space);
    delimited(sep_left, parse_kv, sep_right).parse_next(input)
}

fn parse_value(input: &mut &str) -> PResult<JsonValue> {
    alt((
        parse_null.value(JsonValue::Null),
        parse_string.map(JsonValue::String),
        parse_scientific_notation.map(JsonValue::Number),
        parse_number.map(JsonValue::Number),
        parse_boolean.map(JsonValue::Boolean),
        parse_array.map(JsonValue::Array),
        parse_object.map(JsonValue::Object),
    )).parse_next(input)
}

fn parse_integer(input: &mut &str) -> PResult<f64> {
    let opt = opt(one_of(|c| c == '+' || c == '-')).parse_next(input)?;
    let num = digit1.parse_to::<f64>().parse_next(input)?;

    match opt {
        Some('+') => Ok(num),
        Some('-') => Ok(-num),
        _ => Ok(num)
    }
}

fn parse_scientific_notation(input: &mut &str) -> PResult<f64> {
    let ret = seq!(parse_number, "e", parse_integer).parse_next(input);

    match ret {
        Ok((x, _, z)) => {
            let v = x * 10_f64.powi(z as i32);
            Ok(v)
        },
        Err(e) => Err(e) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_null_should_work() {
        let input = "null";
        let ret = parse_null(&mut (&*input)).unwrap();
        assert_eq!(ret, ());
    }

    #[test]
    fn parse_string_should_work() {
        let input = "\"hello\"";
        let ret = parse_string(&mut (&*input)).unwrap();
        assert_eq!(ret, "hello".to_string());
    }

    #[test]
    fn parse_number_should_work() {
        let input = "123.456789";
        let ret = parse_number(&mut (&*input)).unwrap();
        assert_eq!(ret, 123.456789);
    }

    #[test]
    fn parse_scientific_notation_should_work() {
        let input = "1.1e-30";
        let ret = parse_scientific_notation(&mut (&*input)).unwrap();
        assert_eq!(ret, 1.1e-30);

        let input = "1.1e+1";
        let ret = parse_scientific_notation(&mut (&*input)).unwrap();
        assert_eq!(ret, 1.1e1);
    }

    #[test]
    fn parse_boolean_should_work() {
        let input = "true";
        let ret = parse_boolean(&mut (&*input)).unwrap();
        assert_eq!(ret, true);

        let input = "false";
        let ret = parse_boolean(&mut (&*input)).unwrap();
        assert_eq!(ret, false);
    }

    #[test]
    fn parse_array_should_work() {
        let input = "[1, 2, 3]";
        let ret = parse_array(&mut (&*input)).unwrap();
        assert_eq!(ret, vec![JsonValue::Number(1.0), JsonValue::Number(2.0), JsonValue::Number(3.0)]);
    }

    #[test]
    fn parse_object_should_work() {
        let input = r#"{"key": 1}"#;
        let ret = parse_object(&mut (&*input)).unwrap();
        let mut map = HashMap::new();
        map.insert("key".to_string(), JsonValue::Number(1.0));
        assert_eq!(ret, map);
    }
}
