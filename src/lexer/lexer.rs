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
    keywords: HashMap<String, TokenType>,
    pub(crate) tokens: Vec<Token>,
}
// имплементация
impl<'filename> Lexer<'filename> {
    pub fn new(code: &str, filename: &'filename str) -> Lexer<'filename> {
        let map = HashMap::from([
            (String::from("fun"), TokenType::Fun),
            (String::from("break"), TokenType::Break),
            (String::from("if"), TokenType::If),
            (String::from("elif"), TokenType::Elif),
            (String::from("else"), TokenType::Else),
            (String::from("and"), TokenType::And),
            (String::from("or"), TokenType::Or),
            (String::from("import"), TokenType::Import),
            (String::from("type"), TokenType::Type),
            (String::from("new"), TokenType::New),
            (String::from("match"), TokenType::Match),
            (String::from("case"), TokenType::Case),
            (String::from("default"), TokenType::Default),
            (String::from("lambda"), TokenType::Lambda),
            (String::from("while"), TokenType::While),
            (String::from("unit"), TokenType::Unit),
            (String::from("for"), TokenType::For),
            (String::from("in"), TokenType::In),
            (String::from("continue"), TokenType::Continue),
            (String::from("true"), TokenType::Bool),
            (String::from("false"), TokenType::Bool),
            (String::from("null"), TokenType::Null),
            (String::from("return"), TokenType::Ret),
            (String::from("trait"), TokenType::Trait),
            (String::from("impl"), TokenType::Impl),
            (String::from("native"), TokenType::Native),
            (String::from("impls"), TokenType::Impls),
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
                        self.add_tk(TokenType::AssignAdd, "+=".to_string());
                    } else {
                        self.add_tk(TokenType::Op, "+".to_string());
                    }
                }
                '-' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::AssignSub, "-=".to_string());
                    } else if self.is_match('>') {
                        self.add_tk(TokenType::Arrow, "->".to_string());
                    } else {
                        self.add_tk(TokenType::Op, "-".to_string());
                    }
                }
                '*' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::AssignMul, "*=".to_string());
                    } else {
                        self.add_tk(TokenType::Op, "*".to_string());
                    }
                }
                '%' => {
                    self.add_tk(TokenType::Op, "%".to_string());
                }
                '/' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::AssignDiv, "/=".to_string());
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
                        self.add_tk(TokenType::Op, "/".to_string());
                    }
                }
                '(' => {
                    self.add_tk(TokenType::Lparen, "(".to_string());
                }
                ')' => {
                    self.add_tk(TokenType::Rparen, ")".to_string());
                }
                '{' => {
                    self.add_tk(TokenType::Lbrace, "{".to_string());
                }
                '}' => {
                    self.add_tk(TokenType::Rbrace, "}".to_string());
                }
                '[' => {
                    self.add_tk(TokenType::Lbracket, "[".to_string());
                }
                ']' => {
                    self.add_tk(TokenType::Rbracket, "]".to_string());
                }
                ',' => {
                    self.add_tk(TokenType::Comma, ",".to_string());
                }
                '.' => {
                    if self.is_match('.') {
                        self.add_tk(TokenType::Range, "..".to_string());
                    } else {
                        self.add_tk(TokenType::Dot, ".".to_string());
                    }
                }
                '?' => {
                    self.add_tk(TokenType::Question, "?".to_string());
                }
                ':' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::Walrus, ":=".to_string());
                    } else {
                        self.add_tk(TokenType::Colon, ":".to_string())
                    }
                }
                '<' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::LessEq, "<=".to_string());
                    } else {
                        self.add_tk(TokenType::Less, "<".to_string());
                    }
                }
                '>' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::GreaterEq, ">=".to_string());
                    } else {
                        self.add_tk(TokenType::Greater, ">".to_string());
                    }
                }
                '!' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::NotEq, "!=".to_string());
                    } else {
                        self.add_tk(TokenType::Bang, "!".to_string());
                    }
                }
                '=' => {
                    if self.is_match('=') {
                        self.add_tk(TokenType::Eq, "==".to_string());
                    } else {
                        self.add_tk(TokenType::Assign, "=".to_string());
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
        let tk_type: TokenType = match self.keywords.get(&text) {
            Some(tk_type) => tk_type.clone(),
            None => TokenType::Id,
        };
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
        let mut line_text = "".to_string();
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
        if self.is_at_end() {
            false
        } else {
            if self.char_at(0) == ch {
                self.advance();
                true
            } else {
                false
            }
        }
    }

    fn add_tk(&mut self, tk_type: TokenType, tk_value: String) {
        self.tokens.push(Token::new(
            tk_type,
            tk_value,
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
