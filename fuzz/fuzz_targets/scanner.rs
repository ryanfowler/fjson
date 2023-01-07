#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let scanner = fjson::scanner::Scanner::new(input);
        for res in scanner {
            match res {
                Err(_) => return,
                Ok(event) => match event.token {
                    fjson::scanner::Token::Newline => assert_eq!(&input[event.range], "\n"),
                    fjson::scanner::Token::ObjectStart => assert_eq!(&input[event.range], "{"),
                    fjson::scanner::Token::ObjectEnd => assert_eq!(&input[event.range], "}"),
                    fjson::scanner::Token::ArrayStart => assert_eq!(&input[event.range], "["),
                    fjson::scanner::Token::ArrayEnd => assert_eq!(&input[event.range], "]"),
                    fjson::scanner::Token::Comma => assert_eq!(&input[event.range], ","),
                    fjson::scanner::Token::Colon => assert_eq!(&input[event.range], ":"),
                    fjson::scanner::Token::Null => assert_eq!(&input[event.range], "null"),
                    fjson::scanner::Token::LineComment(v) => {
                        assert_eq!(&input[event.range], ["//", v].join(""))
                    }
                    fjson::scanner::Token::BlockComment(v) => {
                        assert_eq!(&input[event.range], ["/*", v, "*/"].join(""))
                    }
                    fjson::scanner::Token::String(v) => {
                        assert_eq!(&input[event.range], ["\"", v, "\""].join(""))
                    }
                    fjson::scanner::Token::Number(v) => assert_eq!(&input[event.range], v),
                    fjson::scanner::Token::Bool(v) => {
                        assert_eq!(&input[event.range], if v { "true" } else { "false" })
                    }
                },
            }
        }
    }
});
