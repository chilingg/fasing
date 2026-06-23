pub use xprs::Xprs as Exps;

use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error {
    Parse(String),
    Operation(String),
    MissingParameter(String),
}

impl std::fmt::Display for Error {
    #[inline]
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(str) => write!(fmt, "{}", str),
            Self::Operation(str) => write!(
                fmt,
                "The number of parameters must be 2, and the number of operators must be 1: {}",
                str
            ),
            Self::MissingParameter(name) => write!(fmt, "Missing parameter `{}`!", name),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Ordering {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
}

#[derive(Debug, Clone)]
enum Parameter {
    Number(isize),
    Var(String),
}

#[derive(Debug, Clone)]
enum Token {
    Par(Parameter),
    Cmp(Ordering),
}

struct Tokenizer<'a> {
    tokens: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Tokenizer<'a> {
    fn scan_number(&mut self) -> Result<Token, Error> {
        let mut num = String::new();
        while let Some(&c) = self.tokens.peek() {
            if c.is_numeric() {
                num.push(c);
                self.tokens.next();
            } else {
                break;
            }
        }

        match num.parse() {
            Ok(n) => Ok(Token::Par(Parameter::Number(n))),
            Err(e) => Err(Error::Parse(e.to_string())),
        }
    }

    fn scan_var_name(&mut self) -> Result<Token, Error> {
        let mut var = String::new();
        while let Some(&c) = self.tokens.peek() {
            if c.is_ascii_lowercase() || c == '_' {
                var.push(c);
                self.tokens.next();
            } else {
                break;
            }
        }

        Ok(Token::Par(Parameter::Var(var)))
    }

    fn scan_symbol(&mut self) -> Result<Token, Error> {
        match *self.tokens.peek().unwrap() {
            '<' | '>' => {
                let ops = self.tokens.next().unwrap();
                if self.tokens.peek().map(|c| '='.eq(c)).unwrap_or(false) {
                    self.tokens.next().unwrap();
                    match ops {
                        '<' => Ok(Token::Cmp(Ordering::LessEqual)),
                        '>' => Ok(Token::Cmp(Ordering::GreaterEqual)),
                        _ => unreachable!(),
                    }
                } else {
                    match ops {
                        '<' => Ok(Token::Cmp(Ordering::Less)),
                        '>' => Ok(Token::Cmp(Ordering::Greater)),
                        _ => unreachable!(),
                    }
                }
            }
            '=' => {
                self.tokens.next().unwrap();
                if let Some(c) = self.tokens.peek()
                    && '='.eq(c)
                {
                    self.tokens.next().unwrap();
                };
                Ok(Token::Cmp(Ordering::Equal))
            }
            c if c.is_ascii_lowercase() => self.scan_var_name(),
            c => Err(Error::Parse(format!("Invalid character `{c}`"))),
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(c) = self.tokens.peek() {
            if c.is_whitespace() {
                self.tokens.next();
                continue;
            }
            break;
        }

        match self.tokens.peek() {
            Some(c) => {
                if c.is_numeric() {
                    Some(self.scan_number())
                } else {
                    Some(self.scan_symbol())
                }
            }
            None => return None,
        }
    }
}

pub struct BoolExps {
    var1: Parameter,
    var2: Parameter,
    operator: Ordering,
}

impl TryFrom<&str> for BoolExps {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Error> {
        let mut iter = Tokenizer {
            tokens: value.chars().peekable(),
        };

        let var1 = match iter.next() {
            Some(Ok(Token::Par(var1))) => var1,
            _ => Err(Error::Operation(value.to_string()))?,
        };

        let operator = match iter.next() {
            Some(Ok(Token::Cmp(operator))) => operator,
            _ => Err(Error::Operation(value.to_string()))?,
        };

        let var2 = match iter.next() {
            Some(Ok(Token::Par(var1))) => var1,
            _ => Err(Error::Operation(value.to_string()))?,
        };

        Ok(Self {
            var1,
            var2,
            operator,
        })
    }
}

impl BoolExps {
    pub fn eval(&self, variables: &HashMap<&str, isize>) -> Result<bool, Error> {
        let var1 = match &self.var1 {
            Parameter::Number(n) => *n,
            Parameter::Var(name) => *variables
                .get(name.as_str())
                .ok_or(Error::MissingParameter(name.clone()))?,
        };
        let var2 = match &self.var2 {
            Parameter::Number(n) => *n,
            Parameter::Var(name) => *variables
                .get(name.as_str())
                .ok_or(Error::MissingParameter(name.clone()))?,
        };

        let r = match self.operator {
            Ordering::Equal => var1.eq(&var2),
            Ordering::Less => var1.lt(&var2),
            Ordering::LessEqual => var1.le(&var2),
            Ordering::Greater => var1.gt(&var2),
            Ordering::GreaterEqual => var1.ge(&var2),
        };
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr() {
        let exps = "(r+b)*0.6";
        let xprs = Exps::try_from(exps).unwrap();
        match xprs.eval(&[("r", 3.0), ("b", 2.0)].into()) {
            Ok(r) => println!("{exps} = {r}"),
            Err(e) => eprintln!("{e}"),
        }

        let exps = "min((r+b)*0.6, 0)";
        let xprs = Exps::try_from(exps).unwrap();
        match xprs.eval(&[("r", 3.0), ("b", 2.0)].into()) {
            Ok(r) => println!("{exps} = {r}"),
            Err(e) => eprintln!("{e}"),
        }
    }

    #[test]
    fn test_bool_expr() {
        let exps = "1 > 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(!xprs.eval(&[].into()).unwrap());
        let exps = "1 < 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(xprs.eval(&[].into()).unwrap());

        let exps = "1 >= 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(!xprs.eval(&[].into()).unwrap());
        let exps = "1 <= 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(xprs.eval(&[].into()).unwrap());
        let exps = "2 <= 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(xprs.eval(&[].into()).unwrap());
        let exps = "2 >= 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(xprs.eval(&[].into()).unwrap());

        let exps = "1 == 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(!xprs.eval(&[].into()).unwrap());
        let exps = "2 == 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(xprs.eval(&[].into()).unwrap());

        let exps = "1 == 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(!xprs.eval(&[("a", 1), ("b", 2)].into()).unwrap());
        let exps = "2 == 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(xprs.eval(&[("a", 2), ("b", 2)].into()).unwrap());
        let exps = "2 == 2";
        let xprs = BoolExps::try_from(exps).unwrap();
        assert!(xprs.eval(&[("a_1", 2), ("b_", 2)].into()).unwrap());
    }
}
