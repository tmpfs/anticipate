use crate::{
    error::LexError, interpreter::ScriptSource, resolve_path, Error, Result,
};
use logos::{Lexer, Logos};
use std::{
    borrow::Cow,
    ops::Range,
    path::{Path, PathBuf},
};

fn pragma(lex: &mut Lexer<Token>) -> Option<String> {
    let slice = lex.slice();
    let value = &slice[2..];
    Some(value.to_owned())
}

fn integer(lex: &mut Lexer<Token>) -> Option<u64> {
    let slice = lex.slice();
    if let Some(num) = slice.split(' ').last() {
        num.parse().ok()
    } else {
        None
    }
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(error = LexError)]
enum Token {
    #[regex("#![^\n]+", callback = pragma)]
    Pragma(String),
    #[regex("#[$]\\s+sendline\\s")]
    SendLine,
    #[regex("#[$]\\s+sendcontrol\\s")]
    SendControl,
    #[regex("#[$]\\s+expect\\s")]
    Expect,
    #[regex("#[$]\\s+regex\\s")]
    Regex,
    #[regex("#[$]\\s+sleep\\s+([0-9]+)", callback = integer)]
    Sleep(u64),
    #[regex("#[$]\\s+readline\\s*")]
    ReadLine,
    #[regex("#[$]\\s+wait\\s*")]
    Wait,
    #[regex("#[$]\\s+clear\\s*")]
    Clear,
    #[regex("#[$]\\s+send ")]
    Send,
    #[regex("#[$]\\s+flush\\s*")]
    Flush,
    #[regex("#[$]\\s+include\\s+")]
    Include,
    #[regex("#[$].?", priority = 4)]
    Command,
    #[regex("\r?\n", priority = 3)]
    Newline,
    #[regex("(\t| )*#[^$!]?#*.", priority = 2)]
    Comment,
    #[regex("(.|[\t ]+)", priority = 0)]
    Text,
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(error = LexError)]
enum EnvVars {
    #[regex("[$][a-zA-Z0-9_]+")]
    Var,
    #[regex(".", priority = 0)]
    Text,
}

/// Include reference.
#[derive(Debug)]
pub struct Include {
    /// Path to the file.
    pub path: PathBuf,
    /// Index in the parent instructions.
    pub index: usize,
}

/// Instruction to execute.
#[derive(Debug)]
pub enum Instruction<'s> {
    /// Program to execute.
    Pragma(String),
    /// Send a line of text.
    SendLine(&'s str),
    /// Send a control character.
    SendControl(&'s str),
    /// Expect a string.
    Expect(&'s str),
    /// Expect a regex match.
    Regex(&'s str),
    /// Sleep a while.
    Sleep(u64),
    /// Comment text.
    Comment(&'s str),
    /// Read a line of output.
    ReadLine,
    /// Wait for the prompt.
    Wait,
    /// Clear the screen.
    Clear,
    /// Send text, the output stream is not flushed.
    Send(&'s str),
    /// Flush the output stream.
    Flush,
    /// Include script.
    Include(ScriptSource),
}

/// Sequence of commands to execute.
pub type Instructions<'s> = Vec<Instruction<'s>>;

/// Parser for scripts.
#[derive(Debug)]
pub struct ScriptParser;

impl ScriptParser {
    /// Parse input commands.
    pub fn parse(source: &str) -> Result<Instructions<'_>> {
        let (instructions, _) = ScriptParser::parse_file(source, "")?;
        Ok(instructions)
    }

    /// Parse input commands relative to a file path.
    pub fn parse_file(
        source: &str,
        base: impl AsRef<Path>,
    ) -> Result<(Instructions<'_>, Vec<Include>)> {
        let mut cmd = Vec::new();
        let mut lex = Token::lexer(source);
        let mut next_token = lex.next();
        let mut includes = Vec::new();
        while let Some(token) = next_token.take() {
            let token = token?;
            let span = lex.span();
            tracing::debug!(token = ?token, "parse");
            match token {
                Token::Command => {
                    let (text, _) = Self::parse_text(&mut lex, source, None)?;
                    return Err(Error::UnknownInstruction(text.to_owned()));
                }
                Token::Comment => {
                    let (_, finish) =
                        Self::parse_text(&mut lex, source, None)?;
                    let text = &source[span.start..finish.end];
                    cmd.push(Instruction::Comment(text));
                }
                Token::Include => {
                    let (text, _) = Self::parse_text(&mut lex, source, None)?;
                    let text = text.trim();
                    match resolve_path(base.as_ref(), text) {
                        Ok(path) => {
                            let path: PathBuf = path.as_ref().into();
                            if !path.try_exists()? {
                                return Err(Error::Include(
                                    text.to_owned(),
                                    path,
                                ));
                            }
                            includes.push(Include {
                                index: cmd.len(),
                                path,
                            });
                        }
                        Err(_) => {
                            return Err(Error::Include(
                                text.to_owned(),
                                PathBuf::from(text),
                            ));
                        }
                    }
                }
                Token::ReadLine => {
                    cmd.push(Instruction::ReadLine);
                }
                Token::Wait => {
                    cmd.push(Instruction::Wait);
                }
                Token::Clear => {
                    cmd.push(Instruction::Clear);
                }
                Token::Pragma(pragma) => {
                    if !cmd.is_empty() {
                        return Err(Error::PragmaFirst);
                    }
                    cmd.push(Instruction::Pragma(pragma));
                }
                Token::Send => {
                    let (text, _) = Self::parse_text(&mut lex, source, None)?;
                    cmd.push(Instruction::Send(text));
                }
                Token::Flush => {
                    cmd.push(Instruction::Flush);
                }
                Token::SendLine => {
                    let (text, _) = Self::parse_text(&mut lex, source, None)?;
                    cmd.push(Instruction::SendLine(text));
                }
                Token::Expect => {
                    let (text, _) = Self::parse_text(&mut lex, source, None)?;
                    cmd.push(Instruction::Expect(text));
                }
                Token::Regex => {
                    let (text, _) = Self::parse_text(&mut lex, source, None)?;
                    cmd.push(Instruction::Regex(text));
                }
                Token::SendControl => {
                    let (text, _) = Self::parse_text(&mut lex, source, None)?;
                    cmd.push(Instruction::SendControl(text));
                }
                Token::Sleep(num) => {
                    cmd.push(Instruction::Sleep(num));
                }
                // Unhandled text is send line
                Token::Text => {
                    let (text, _) =
                        Self::parse_text(&mut lex, source, Some(span))?;
                    if text.starts_with("#$") {
                        return Err(Error::UnknownInstruction(
                            text.to_owned(),
                        ));
                    }
                    cmd.push(Instruction::SendLine(text));
                }
                Token::Newline => {}
            }
            next_token = lex.next();
        }

        Ok((cmd, includes))
    }

    fn parse_text<'s>(
        lex: &mut Lexer<Token>,
        source: &'s str,
        start: Option<Range<usize>>,
    ) -> Result<(&'s str, Range<usize>)> {
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
                Token::Text => {
                    finish = lex.span();
                }
                _ => break,
            }
            next_token = lex.next();
        }
        Ok((&source[begin.start..finish.end], finish))
    }

    pub(crate) fn interpolate(value: &str) -> Result<Cow<str>> {
        if value.contains('$') {
            let mut s = String::new();
            let mut lex = EnvVars::lexer(value);
            let mut next_token = lex.next();
            while let Some(token) = next_token.take() {
                let token = token?;
                match token {
                    EnvVars::Var => {
                        let var = lex.slice();
                        if let Ok(val) = std::env::var(&var[1..]) {
                            s.push_str(&val);
                        } else {
                            s.push_str(var);
                        }
                    }
                    _ => s.push_str(lex.slice()),
                }

                next_token = lex.next();
            }
            Ok(Cow::Owned(s))
        } else {
            Ok(Cow::Borrowed(value))
        }
    }
}
