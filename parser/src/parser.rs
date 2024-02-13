use logos::{Logos, Lexer};
use crate::{Result, Error, error::LexError};
use std::ops::Range;

#[derive(Logos, Debug, PartialEq, Copy, Clone)]
#[logos(error = LexError)]
enum Token {
    #[regex("#[$]\\s+sendline\\s+")]
    SendLine,
    #[regex("#[$]\\s+sendcontrol\\s+")]
    SendControl,
    #[regex("#[$]\\s+expect\\s+")]
    Expect,
    #[regex("#[$]\\s+regex\\s+")]
    Regex,
    #[regex("\r?\n")]
    Newline,
    #[regex(".", priority = 0)]
    Text,
}

type LexResult<T> = std::result::Result<T, LexError>;

/// Command to execute.
#[derive(Debug)]
pub enum Command<'s> {
    /// Send a line of text.
    SendLine(&'s str),
    /// Send a control character.
    SendControl(char),
    /// Expect a string.
    Expect(&'s str),
    /// Expect a regex match.
    Regex(&'s str),
}

/// Sequence of commands to execute.
#[derive(Debug, Default)]
pub struct Commands<'s> {
    commands: Vec<Command<'s>>,
}

impl<'s> Commands<'s> {
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Command<'s>> {
        self.commands.iter()
    }
}

pub struct CommandParser<'s> {
    source: &'s str,
}

impl<'s> CommandParser<'s> {
    /// Create a new parser.
    pub fn new(source: &'s str) -> Self {
        Self { source }
    }

    /// Get a lex for the current source.
    fn lex(&self) -> Lexer<'s, Token> {
        Token::lexer(self.source)
    }
    
    /// Parse input commands.
    pub fn parse(&self) -> Result<Commands> {
        
        let mut cmd: Commands = Default::default();

        let mut lex = self.lex();
        let mut next_token = lex.next();
        while let Some(token) = next_token.take() {
            let token = token?;
            println!("token {:#?}", token);

            match token {
                Token::SendLine => {
                    let text = self.parse_text(&mut lex)?;
                    cmd.commands.push(Command::SendLine(text));
                }
                Token::Expect => {
                    let text = self.parse_text(&mut lex)?;
                    cmd.commands.push(Command::Expect(text));
                }
                Token::Regex => {
                    let text = self.parse_text(&mut lex)?;
                    cmd.commands.push(Command::Regex(text));
                }
                Token::SendControl => {
                    let text = self.parse_text(&mut lex)?;
                    let mut it = text.chars();
                    if let Some(c) = it.next() {
                        cmd.commands.push(Command::SendControl(c));
                        if it.next().is_some() {
                            panic!("too many characters");
                        }
                    }
                }
                _ => {}
            }
            next_token = lex.next();
        }

        Ok(cmd)
    }

    fn parse_text(
        &self,
        lex: &mut Lexer<Token>,
    ) -> Result<&'s str> {
        let mut begin = lex.span().end..lex.span().end;
        let mut finish: Range<usize> = lex.span();
        let mut next_token = lex.next();
        while let Some(token) = next_token.take() {
            let token = token?;
            if let Token::Text = token {
                finish = lex.span();
            } else {
                break;
            }
            next_token = lex.next();
        }
        Ok(&self.source[begin.start..finish.end])
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use super::CommandParser;

    #[test]
    fn parse_sendline() -> Result<()> {
        let source = r#"
#$ sendline foo
#$ expect bar
#$ regex [0-9]
#$ sendcontrol c
"#;
        let parser = CommandParser::new(source);
        let commands = parser.parse()?;
        println!("{:#?}", commands);
        Ok(())
    }
}

