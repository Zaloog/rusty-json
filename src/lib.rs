use pyo3::prelude::*;
mod parser;

/// A Python module implemented in Rust.
#[pymodule]
mod rusty_json {
    use pyo3::prelude::*;
    use crate::parser::{parse_tokens, tokenize, JsonValue};

    /// Formats the sum of two numbers as string.
    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
        Ok((a + b).to_string())
    }

    #[pyfunction]
    fn load_json(json_str: String) -> PyResult<JsonValue> {
        let tokens = tokenize(json_str).unwrap();
        let json_value = parse_tokens(tokens).unwrap();
        // println!("{:?}", json_value);

        Ok(json_value)
    }
}
