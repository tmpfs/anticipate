use crate::{error::LexError, Error, Result};
use logos::{Lexer, Logos};
use std::ops::Range;

fn pragma(lex: &mut Lexer<Token>) -> Option<String> {
    let slice = lex.slice();
    let value = &slice[2..];
    Some(value.to_owned())
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(error = LexError)]
enum Token {
    #[regex("#![^\n]+", callback = pragma)]
    Pragma(String),
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
    #[regex("#[^$].*", priority = 1)]
    Comment,
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
    Pragma(String),
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
    /// Comment text.
    Comment(&'s str),
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
                Token::Comment => {
                    let text = self.parse_text(&mut lex, source, None)?;
                    cmd.push(Instruction::Comment(text));
                }
                Token::Pragma(pragma) => {
                    if !cmd.is_empty() {
                        return Err(Error::PragmaFirst);
                    }
                    cmd.push(Instruction::Pragma(pragma));
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
                Token::Number | Token::Newline => {}
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
            match token {
                Token::Text | Token::Number => {
                    finish = lex.span();
                }
                _ => break,
            }
            next_token = lex.next();
        }
        Ok(&source[begin.start..finish.end])
    }
}

