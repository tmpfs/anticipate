use anticipate_core::{ScriptParser, Instruction, Error};
use anyhow::Result;

#[test]
fn parse_pragma() -> Result<()> {
    let source = "#!../programs/foo.sh";
    let instructions = ScriptParser.parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Pragma(_))));
    Ok(())
}

#[test]
fn parse_sendline() -> Result<()> {
    let source = "#$ sendline foo";
    let instructions = ScriptParser.parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::SendLine(_))));
    Ok(())
}

#[test]
fn parse_readline() -> Result<()> {
    let source = "#$ readline";
    let instructions = ScriptParser.parse(source)?;
    println!("{:#?}", instructions);
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::ReadLine)));
    Ok(())
}

#[test]
fn parse_sendline_raw() -> Result<()> {
    let source = "foo";
    let instructions = ScriptParser.parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::SendLine(_))));
    Ok(())
}

#[test]
fn parse_expect() -> Result<()> {
    let source = "#$ expect bar";
    let instructions = ScriptParser.parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Expect(_))));
    Ok(())
}

#[test]
fn parse_regex() -> Result<()> {
    let source = "#$ regex [0-9]";
    let instructions = ScriptParser.parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Regex(_))));
    Ok(())
}

#[test]
fn parse_sendcontrol() -> Result<()> {
    let source = "#$ sendcontrol c";
    let instructions = ScriptParser.parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::SendControl(_))));
    Ok(())
}

#[test]
fn parse_wait() -> Result<()> {
    let source = "#$ wait 500";
    let instructions = ScriptParser.parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Wait(_))));
    Ok(())
}

#[test]
fn parse_comment() -> Result<()> {
    let source = "# this is a comment that does nothing";
    let instructions = ScriptParser.parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Comment(_))));
    Ok(())
}

// Errors

#[test]
fn parse_pragma_first_err() -> Result<()> {
    let source = r#"
# comment before the pragma
#!sh"#;
    let result = ScriptParser.parse(source);
    assert!(matches!(result, Err(Error::PragmaFirst)));
    Ok(())
}

#[test]
fn parse_wait_number() -> Result<()> {
    let source = "#$ wait foo";
    let result = ScriptParser.parse(source);
    assert!(matches!(result, Err(Error::NumberExpected(_))));
    Ok(())
}