use crate::{error::LexError, Error, Result};
use logos::{Lexer, Logos};
use std::ops::Range;

#[derive(Logos, Debug, PartialEq, Copy, Clone)]
#[logos(error = LexError)]
enum Token {
    #[regex("#!")]
    Pragma,
    #[regex("#[$]\\s+sendline\\s+")]
    SendLine,
    #[regex("#[$]\\s+sendcontrol\\s+")]
    SendControl,
    #[regex("#[$]\\s+expect\\s+")]
    Expect,
    #[regex("#[$]\\s+regex\\s+")]
    Regex,
    #[regex("#[$]\\s+wait\\s+")]
    Wait,
    #[regex("[0-9]+")]
    Number,
    #[regex("\r?\n")]
    Newline,
    #[regex(".", priority = 0)]
    Text,
}

/// Instruction to execute.
#[derive(Debug)]
pub enum Instruction<'s> {
    /// Program to execute.
    Pragma(&'s str),
    /// Send a line of text.
    SendLine(&'s str),
    /// Send a control character.
    SendControl(char),
    /// Expect a string.
    Expect(&'s str),
    /// Expect a regex match.
    Regex(&'s str),
    /// Wait a while.
    Wait(u64),
}

/// Sequence of commands to execute.
pub type Instructions<'s> = Vec<Instruction<'s>>;

#[derive(Debug)]
pub struct ScriptParser;

impl ScriptParser {
    /// Parse input commands.
    pub fn parse<'s>(&self, source: &'s str) -> Result<Instructions<'s>> {
        let mut cmd = Vec::new();
        let mut lex = Token::lexer(source);
        let mut next_token = lex.next();
        while let Some(token) = next_token.take() {
            let token = token?;
            let span = lex.span();
            tracing::trace!(token = ?token, "parse");

            match token {
                Token::Pragma => {
                    let text = self.parse_text(&mut lex, source, None)?;
                    cmd.push(Instruction::Pragma(text));
                }
                Token::SendLine => {
                    let text = self.parse_text(&mut lex, source, None)?;
                    cmd.push(Instruction::SendLine(text));
                }
                Token::Expect => {
                    let text = self.parse_text(&mut lex, source, None)?;
                    cmd.push(Instruction::Expect(text));
                }
                Token::Regex => {
                    let text = self.parse_text(&mut lex, source, None)?;
                    cmd.push(Instruction::Regex(text));
                }
                Token::SendControl => {
                    let text = self.parse_text(&mut lex, source, None)?;
                    let mut it = text.chars();
                    if let Some(c) = it.next() {
                        cmd.push(Instruction::SendControl(c));
                        if it.next().is_some() {
                            panic!("too many characters");
                        }
                    }
                }
                Token::Wait => {
                    let num = self.parse_number(&mut lex, source)?;
                    cmd.push(Instruction::Wait(num));
                }
                // Unhandled text is send line
                Token::Text => {
                    let text =
                        self.parse_text(&mut lex, source, Some(span))?;
                    cmd.push(Instruction::SendLine(text));
                }
                _ => {}
            }
            next_token = lex.next();
        }

        Ok(cmd)
    }

    fn parse_number<'s>(
        &self,
        lex: &mut Lexer<Token>,
        source: &'s str,
    ) -> Result<u64> {
        let next_token = lex.next();
        let span = lex.span();
        let val = &source[span.start..span.end];
        if let Some(Ok(Token::Number)) = next_token {
            Ok(val.parse()?)
        } else {
            Err(Error::NumberExpected(val.to_owned()))
        }
    }

    fn parse_text<'s>(
        &self,
        lex: &mut Lexer<Token>,
        source: &'s str,
        start: Option<Range<usize>>,
    ) -> Result<&'s str> {
        let begin = if let Some(start) = start {
            start
        } else {
            lex.span().end..lex.span().end
        };

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
        Ok(&source[begin.start..finish.end])
    }
}

#[cfg(test)]
mod test {
    use super::ScriptParser;
    use anyhow::Result;

    #[test]
    fn parse_sendline() -> Result<()> {
        let source = r#"
#$ sendline foo
#$ expect bar
#$ regex [0-9]
#$ sendcontrol c
"#;
        let parser = ScriptParser::new(source);
        let commands = parser.parse()?;
        println!("{:#?}", commands);
        Ok(())
    }
}
