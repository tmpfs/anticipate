use logos::{Logos, Lexer};
use crate::{Result, Error, error::LexError};
use std::ops::Range;

#[derive(Logos, Debug, PartialEq, Copy, Clone)]
#[logos(error = LexError)]
enum Token {
    #[regex("#[$]\\s+sendline\\s+")]
    SendLine,
    #[regex("#[$]\\s+expect\\s+")]
    Expect,
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
    /// Expect a line of text.
    Expect(&'s str),
}

/// Sequence of commands to execute.
#[derive(Debug, Default)]
pub struct Commands<'s> {
    commands: Vec<Command<'s>>,
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
"#;
        let parser = CommandParser::new(source);
        let commands = parser.parse()?;
        println!("{:#?}", commands);
        Ok(())
    }
}

