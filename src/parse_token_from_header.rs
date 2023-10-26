static PARSE_ERROR_MESSAGE: &str = "Authorization token must start with 'Bearer '";

pub fn parse_token_from_header(authorization_token: &str) -> Result<&str, &'static str> {
    if authorization_token.len() >= 8 && &(authorization_token[0..7]) == "Bearer " {
        return Ok(&(authorization_token[7..]));
    }
    Err(PARSE_ERROR_MESSAGE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_a_token_from_a_valid_header() {
        let result = parse_token_from_header("Bearer sometoken");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sometoken");
    }

    #[test]
    fn it_should_fail_to_parse_a_empty_header() {
        let result = parse_token_from_header("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PARSE_ERROR_MESSAGE);
    }

    #[test]
    fn it_should_fail_to_parse_a_header_containing_a_string_shorter_than_bearer() {
        let result = parse_token_from_header("short");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PARSE_ERROR_MESSAGE);
    }

    #[test]
    fn it_should_fail_to_parse_a_header_that_does_not_start_with_bearer() {
        let result = parse_token_from_header("NotBearer sometoken");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PARSE_ERROR_MESSAGE);
    }
}
