pub fn is_valid_json(json: &str) -> bool {
    let chars: Vec<char> = json.chars().collect();
    let mut index = 0;

    fn skip_whitespace(chars: &[char], index: &mut usize) {
        while *index < chars.len() && chars[*index].is_ascii_whitespace() {
            *index += 1;
        }
    }

    fn parse_value(chars: &[char], index: &mut usize) -> bool {
        skip_whitespace(chars, index);
        if *index >= chars.len() {
            return false;
        }

        match chars[*index] {
            '{' => parse_object(chars, index),
            '[' => parse_array(chars, index),
            '"' => parse_string(chars, index),
            't' | 'f' => {
                parse_literal(chars, index, "true") || parse_literal(chars, index, "false")
            }
            'n' => parse_literal(chars, index, "null"),
            '-' | '0'..='9' => parse_number(chars, index),
            _ => false,
        }
    }

    fn parse_literal(chars: &[char], index: &mut usize, literal: &str) -> bool {
        let expected: Vec<char> = literal.chars().collect();
        if *index + expected.len() > chars.len() {
            return false;
        }
        for (i, &c) in expected.iter().enumerate() {
            if chars[*index + i] != c {
                return false;
            }
        }
        *index += expected.len();
        true
    }

    fn parse_string(chars: &[char], index: &mut usize) -> bool {
        if chars[*index] != '"' {
            return false;
        }
        *index += 1;
        let mut escaped = false;
        while *index < chars.len() {
            let c = chars[*index];
            *index += 1;

            if escaped {
                escaped = false;
                continue;
            }

            if c == '\\' {
                escaped = true;
            } else if c == '"' {
                return true;
            }
        }
        false
    }

    fn parse_number(chars: &[char], index: &mut usize) -> bool {
        let start = *index;
        if chars[*index] == '-' {
            *index += 1;
        }
        let mut has_digits = false;
        while *index < chars.len() && chars[*index].is_ascii_digit() {
            *index += 1;
            has_digits = true;
        }
        if *index < chars.len() && chars[*index] == '.' {
            *index += 1;
            let mut has_frac = false;
            while *index < chars.len() && chars[*index].is_ascii_digit() {
                *index += 1;
                has_frac = true;
            }
            if !has_frac {
                return false;
            }
        }
        *index > start && has_digits
    }

    fn parse_array(chars: &[char], index: &mut usize) -> bool {
        *index += 1;
        skip_whitespace(chars, index);

        if *index < chars.len() && chars[*index] == ']' {
            *index += 1;
            return true;
        }

        loop {
            if !parse_value(chars, index) {
                return false;
            }
            skip_whitespace(chars, index);

            if *index >= chars.len() {
                return false;
            }
            if chars[*index] == ']' {
                *index += 1;
                return true;
            } else if chars[*index] == ',' {
                *index += 1;
            } else {
                return false;
            }
        }
    }

    fn parse_object(chars: &[char], index: &mut usize) -> bool {
        *index += 1;
        skip_whitespace(chars, index);

        if *index < chars.len() && chars[*index] == '}' {
            *index += 1;
            return true;
        }

        loop {
            skip_whitespace(chars, index);
            if !parse_string(chars, index) {
                return false;
            }
            skip_whitespace(chars, index);

            if *index >= chars.len() || chars[*index] != ':' {
                return false;
            }
            *index += 1;
            if !parse_value(chars, index) {
                return false;
            }
            skip_whitespace(chars, index);

            if *index >= chars.len() {
                return false;
            }
            if chars[*index] == '}' {
                *index += 1;
                return true;
            } else if chars[*index] == ',' {
                *index += 1;
            } else {
                return false;
            }
        }
    }

    if !parse_value(&chars, &mut index) {
        return false;
    }
    skip_whitespace(&chars, &mut index);
    index == chars.len()
}
