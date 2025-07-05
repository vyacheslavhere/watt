// импорты
use crate::error;
use crate::errors::errors::Error;
use crate::lexer::address::*;
use std::collections::HashMap;

// тип токена
#[derive(Debug, Clone, Eq, PartialEq, Copy, Hash)]
#[allow(dead_code)]
pub enum TokenType {
    Fun,
    Op,        // +, -, *, /
    Lparen,    // (
    Rparen,    // )
    Lbrace,    // {
    Rbrace,    // }
    Lambda,    // lambda
    Walrus,    // :=
    Eq,        // ==
    NotEq,     // !=
    Text,      // 'text'
    Number,    // 1234567890.0123456789
    Assign,    // =
    Id,        // variable id
    Comma,     // ,
    Ret,       // return
    If,        // if
    Bool,      // bool
    While,     // while
    Type,      // type
    New,       // new
    Dot,       // dot
    Greater,   // >
    Less,      // <
    GreaterEq, // >=
    LessEq,    // <=
    Null,      // null
    Elif,      // elif
    Else,      // else
    And,       // logical and
    Or,        // logical or
    Import,    // import
    AssignAdd, // assign add
    AssignSub, // assign sub
    AssignMul, // assign mul
    AssignDiv, // assign divide
    Break,     // break
    Match,     // match
    Case,      // case
    Default,   // default
    Lbracket,  // [
    Rbracket,  // ]
    Colon,     // colon :
    For,       // for
    Bang,      // !
    In,        // in
    Continue,  // continue
    Arrow,     // ->
    Unit,      // unit
    Native,    // native
    With,      // with
    Trait,     // trait
    Impl,      // impl
    Question,  // ?
    Impls,     // impls
    Range,     // ..
}

// токен
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Token {
    pub tk_type: TokenType,
    pub value: String,
    pub address: Address,
}
// имплементация
impl Token {
    pub fn new(tk_type: TokenType, value: String, address: Address) -> Token {
        Token {
            tk_type,
            value,
            address,
        }
    }
}

// лексер
pub struct Lexer<'filename> {
    line: u64,
    column: u16,
    current: u128,
    line_text: String,
    code: Vec<char>,
    filename: &'filename str,
    pub tokens: Vec<Token>,
    keywords: HashMap<&'static str, TokenType>,
}

// имплементация
impl<'filename> Lexer<'filename> {
    pub fn new(code: &str, filename: &'filename str) -> Lexer<'filename> {
        let map = HashMap::from([
            ("fun", TokenType::Fun),
            ("break", TokenType::Break),
            ("if", TokenType::If),
            ("elif", TokenType::Elif),
            ("else", TokenType::Else),
            ("and", TokenType::And),
            ("or", TokenType::Or),
            ("import", TokenType::Import),
            ("type", TokenType::Type),
            ("new", TokenType::New),
            ("match", TokenType::Match),
            ("case", TokenType::Case),
            ("default", TokenType::Default),
            ("lambda", TokenType::Lambda),
            ("while", TokenType::While),
            ("unit", TokenType::Unit),
            ("for", TokenType::For),
            ("in", TokenType::In),
            ("continue", TokenType::Continue),
            ("true", TokenType::Bool),
            ("false", TokenType::Bool),
            ("null", TokenType::Null),
            ("return", TokenType::Ret),
            ("trait", TokenType::Trait),
            ("impl", TokenType::Impl),
            ("native", TokenType::Native),
            ("impls", TokenType::Impls),
        ]);
        // лексер
        let mut lexer = Lexer {
            line: 1,
            current: 0,
            column: 0,
            line_text: String::new(),
            code: code.chars().collect::<Vec<char>>(),
            filename,
            tokens: vec![],
            keywords: map,
        };
        // текст первой линии
        lexer.line_text = lexer.get_line_text();
        // возвращаем
        lexer
    }

    pub fn lex(&mut self) {
        self.tokens.clear();

        while !self.is_at_end() {
            let ch = self.advance();
            match ch {
                '+' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::AssignAdd, "+=");
                    } else {
                        self.add_tk(TokenType::Op, "+");
                    }
                }
                '-' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::AssignSub, "-=");
                    } else if self.is_match('>') {
                        self.add_tk(TokenType::Arrow, "->");
                    } else {
                        self.add_tk(TokenType::Op, "-");
                    }
                }
                '*' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::AssignMul, "*=");
                    } else {
                        self.add_tk(TokenType::Op, "*");
                    }
                }
                '%' => {
                    self.add_tk(TokenType::Op, "%");
                }
                '/' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::AssignDiv, "/=");
                    } else if self.is_match('/') {
                        while !self.is_match('\n') && !self.is_at_end() {
                            self.advance();
                        }
                        self.newline();
                    } else if self.is_match('*') {
                        while !(self.peek() == '*' && self.next() == '/') && !self.is_at_end() {
                            if self.is_match('\n') {
                                self.newline();
                                continue;
                            }
                            self.advance();
                        }
                        // *
                        self.advance();
                        // /
                        self.advance();
                    } else {
                        self.add_tk(TokenType::Op, "/");
                    }
                }
                '(' => {
                    self.add_tk(TokenType::Lparen, "(");
                }
                ')' => {
                    self.add_tk(TokenType::Rparen, ")");
                }
                '{' => {
                    self.add_tk(TokenType::Lbrace, "{");
                }
                '}' => {
                    self.add_tk(TokenType::Rbrace, "}");
                }
                '[' => {
                    self.add_tk(TokenType::Lbracket, "[");
                }
                ']' => {
                    self.add_tk(TokenType::Rbracket, "]");
                }
                ',' => {
                    self.add_tk(TokenType::Comma, ",");
                }
                '.' => {
                    if self.is_match('.') {
                        self.add_tk(TokenType::Range, "..");
                    } else {
                        self.add_tk(TokenType::Dot, ".");
                    }
                }
                '?' => {
                    self.add_tk(TokenType::Question, "?");
                }
                ':' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::Walrus, ":=");
                    } else {
                        self.add_tk(TokenType::Colon, ":")
                    }
                }
                '<' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::LessEq, "<=");
                    } else {
                        self.add_tk(TokenType::Less, "<");
                    }
                }
                '>' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::GreaterEq, ">=");
                    } else {
                        self.add_tk(TokenType::Greater, ">");
                    }
                }
                '!' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::NotEq, "!=");
                    } else {
                        self.add_tk(TokenType::Bang, "!");
                    }
                }
                '=' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::Eq, "==");
                    } else {
                        self.add_tk(TokenType::Assign, "=");
                    }
                }
                // пробелы
                '\r' => {}
                '\t' => {}
                '\0' => {}
                '\n' => {
                    self.newline();
                }
                ' ' => {}
                '\'' => match self.scan_string() {
                    Ok(tk) => {
                        self.tokens.push(tk);
                    }
                    Err(err) => {
                        error!(err);
                    }
                },
                _ => {
                    if self.is_digit(ch) {
                        match self.scan_number(ch) {
                            Ok(tk) => {
                                self.tokens.push(tk);
                            }
                            Err(err) => {
                                error!(err);
                            }
                        }
                    } else if self.is_id(ch) {
                        let token = self.scan_id_or_keyword(ch);
                        self.tokens.push(token);
                    } else {
                        error!(Error::new(
                            Address::new(
                                self.line,
                                self.column,
                                self.filename.to_string(),
                                self.line_text.clone(),
                            ),
                            format!("unexpected char: {}", ch),
                            format!("delete char: {}", ch),
                        ));
                    }
                }
            }
        }
    }

    fn scan_string(&mut self) -> Result<Token, Error> {
        let mut text: String = String::new();
        while self.peek() != '\'' {
            text.push(self.advance());
            if self.is_at_end() || self.is_match('\n') {
                return Err(Error::new(
                    Address::new(
                        self.line,
                        self.column,
                        self.filename.to_string(),
                        self.line_text.clone(),
                    ),
                    "unclosed string quotes.".to_string(),
                    "did you forget ' symbol?".to_string(),
                ));
            }
        }
        self.advance();
        Ok(Token {
            tk_type: TokenType::Text,
            value: text,
            address: Address::new(
                self.line,
                self.column,
                self.filename.to_string(),
                self.line_text.clone(),
            ),
        })
    }

    fn scan_number(&mut self, start: char) -> Result<Token, Error> {
        let mut text: String = String::from(start);
        let mut is_float: bool = false;
        while self.is_digit(self.peek()) || self.peek() == '.' {
            if self.peek() == '.' {
                if self.next() == '.' {
                    break;
                }
                if is_float {
                    return Err(Error::new(
                        Address::new(
                            self.line,
                            self.column,
                            self.filename.to_string(),
                            self.line_text.clone(),
                        ),
                        "couldn't parse number with two dots".to_string(),
                        "check your code.".to_string(),
                    ));
                }
                is_float = true;
                text.push(self.advance());
                continue;
            }
            text.push(self.advance());
            if self.is_at_end() {
                break;
            }
        }
        Ok(Token {
            tk_type: TokenType::Number,
            value: text,
            address: Address::new(
                self.line,
                self.column,
                self.filename.to_string(),
                self.line_text.clone(),
            ),
        })
    }

    fn scan_id_or_keyword(&mut self, start: char) -> Token {
        let mut text: String = String::from(start);
        
        while self.is_id(self.peek()) {
            text.push(self.advance());
            if self.is_at_end() {
                break;
            }
        }
        
        let tk_type: TokenType = self.keywords.get(text.as_str()).cloned().unwrap_or(TokenType::Id);
        
        Token {
            tk_type,
            value: text,
            address: Address::new(
                self.line,
                self.column,
                self.filename.to_string(),
                self.line_text.clone(),
            ),
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.code.len() as u128
    }

    fn is_at_end_offset(&self, offset: u128) -> bool {
        self.current + offset >= self.code.len() as u128
    }

    fn char_at(&self, offset: u128) -> char {
        let index = (self.current + offset) as usize;
        if self.code.len() > index {
            let c = self.code[index];
            c
        } else {
            '\0'
        }
    }

    fn get_line_text(&self) -> String {
        // проходимся по тексту
        let mut i = 0;
        let mut line_text = String::new();
        while !self.is_at_end_offset(i) && self.char_at(i) != '\n' {
            line_text.push(self.char_at(i));
            i += 1;
        }
        // возвращаем
        line_text
    }

    fn newline(&mut self) {
        self.line += 1;
        self.column = 0;
        self.line_text = self.get_line_text();
    }

    fn advance(&mut self) -> char {
        let ch: char = self.char_at(0);
        self.current += 1;
        self.column += 1;
        ch
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.char_at(0)
        }
    }

    fn next(&self) -> char {
        if self.current + 1 >= self.code.len() as u128 {
            '\0'
        } else {
            self.char_at(1)
        }
    }

    //noinspection ALL
    fn is_match(&mut self, ch: char) -> bool {
        if !self.is_at_end() {
            if self.char_at(0) == ch {
                self.advance();
                true
            }
        }
        false
    }

    fn add_tk(&mut self, tk_type: TokenType, tk_value: &str) {
        self.tokens.push(Token::new(
            tk_type,
            tk_value.to_string(),
            Address::new(
                self.line,
                self.column,
                self.filename.to_string(),
                self.line_text.clone(),
            ),
        ));
    }

    fn is_digit(&self, ch: char) -> bool {
        ch >= '0' && ch <= '9'
    }

    fn is_letter(&self, ch: char) -> bool {
        (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || (ch == '_')
    }

    fn is_id(&self, ch: char) -> bool {
        self.is_letter(ch) || self.is_digit(ch) || (ch == ':' && self.is_id(self.next()))
    }
}
