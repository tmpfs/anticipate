use anticipate_core::{Error, Instruction, ScriptFile, ScriptParser};
use anyhow::Result;

#[test]
fn parse_pragma() -> Result<()> {
    let source = "#!../programs/foo.sh";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Pragma(_))));
    Ok(())
}

#[test]
fn parse_sendline() -> Result<()> {
    let source = "#$ sendline foo";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(
        instructions.first(),
        Some(Instruction::SendLine(_))
    ));
    Ok(())
}

#[test]
fn parse_readline() -> Result<()> {
    let source = "#$ readline";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::ReadLine)));
    Ok(())
}

#[test]
fn parse_sendline_raw() -> Result<()> {
    let source = "foo";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(
        instructions.first(),
        Some(Instruction::SendLine(_))
    ));
    Ok(())
}

#[test]
fn parse_sendline_raw_numeric() -> Result<()> {
    let source = "2";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(
        instructions.first(),
        Some(Instruction::SendLine(_))
    ));
    Ok(())
}

#[test]
fn parse_expect() -> Result<()> {
    let source = "#$ expect bar";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Expect(_))));
    Ok(())
}

#[test]
fn parse_regex() -> Result<()> {
    let source = "#$ regex [0-9]";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Regex(_))));
    Ok(())
}

#[test]
fn parse_sendcontrol() -> Result<()> {
    let source = "#$ sendcontrol c";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(
        instructions.first(),
        Some(Instruction::SendControl(_))
    ));
    Ok(())
}

#[test]
fn parse_sleep() -> Result<()> {
    let source = "#$ sleep 500";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Sleep(_))));
    Ok(())
}

#[test]
fn parse_comment() -> Result<()> {
    let source = "# this is a comment";
    let instructions = ScriptParser::parse(source)?;
    if let Some(Instruction::Comment(text)) = instructions.get(0) {
        assert_eq!(source, *text);
    } else {
        panic!("expected comment instruction");
    }
    Ok(())
}

#[test]
fn parse_send() -> Result<()> {
    let source = "#$ send echo";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Send(_))));
    Ok(())
}

#[test]
fn parse_flush() -> Result<()> {
    let source = "#$ flush";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Flush)));
    Ok(())
}

#[test]
fn parse_wait() -> Result<()> {
    let source = "#$ wait";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Wait)));
    Ok(())
}

#[test]
fn parse_clear() -> Result<()> {
    let source = "#$ clear";
    let instructions = ScriptParser::parse(source)?;
    assert_eq!(1, instructions.len());
    assert!(matches!(instructions.first(), Some(Instruction::Clear)));
    Ok(())
}

#[test]
fn parse_include() -> Result<()> {
    let file = "tests/fixtures/include.sh";
    let file = ScriptFile::parse(file)?;
    let instructions = file.instructions();
    assert_eq!(1, instructions.len());
    if let Some(Instruction::Include(source)) = instructions.get(0) {
        if let Some(Instruction::SendLine(val)) =
            source.borrow_instructions().get(0)
        {
            assert_eq!("echo hi", *val);
        } else {
            panic!("expected send line in include");
        }

        assert!(matches!(
            source.borrow_instructions().get(1),
            Some(Instruction::ReadLine)
        ));

        if let Some(Instruction::SendLine(val)) =
            source.borrow_instructions().get(2)
        {
            assert_eq!("exit", *val);
        } else {
            panic!("expected send line in include");
        }
    } else {
        panic!("expected include instruction");
    }
    Ok(())
}

#[test]
fn parse_include_many() -> Result<()> {
    let file = "tests/fixtures/include-many.sh";
    let file = ScriptFile::parse(file)?;
    let instructions = file.instructions();
    let mut it = instructions.iter();
    assert!(matches!(it.next(), Some(Instruction::Comment(_))));
    assert!(matches!(it.next(), Some(Instruction::SendLine(_))));
    assert!(matches!(it.next(), Some(Instruction::Include(_))));
    assert!(matches!(it.next(), Some(Instruction::Wait)));
    assert!(matches!(it.next(), Some(Instruction::Comment(_))));
    assert!(matches!(it.next(), Some(Instruction::SendLine(_))));
    assert!(matches!(it.next(), Some(Instruction::Include(_))));
    assert!(matches!(it.next(), Some(Instruction::Wait)));
    Ok(())
}

#[test]
fn parse_comment_leading_whitespace() -> Result<()> {
    let source = r#"    # comment with leading whitespace"#;
    let result = ScriptParser::parse(source);
    
    println!("{:#?}", result);

    //assert!(matches!(result, Err(Error::PragmaFirst)));
    Ok(())
}

// Errors

#[test]
fn parse_pragma_first_err() -> Result<()> {
    let source = r#"
# comment before the pragma
#!sh"#;
    let result = ScriptParser::parse(source);
    
    println!("{:#?}", result);

    assert!(matches!(result, Err(Error::PragmaFirst)));
    Ok(())
}

#[test]
fn parse_unknown() -> Result<()> {
    let source = "#$ foobar";
    let result = ScriptParser::parse(source);
    assert!(matches!(result, Err(Error::UnknownInstruction(_))));

    if let Err(Error::UnknownInstruction(cmd)) = result {
        assert_eq!("#$ foobar", cmd);
    } else {
        panic!("expected unknown instruction error");
    }
    Ok(())
}

#[test]
fn parse_unknown_empty() -> Result<()> {
    let source = "#$";
    let result = ScriptParser::parse(source);
    assert!(matches!(result, Err(Error::UnknownInstruction(_))));

    if let Err(Error::UnknownInstruction(cmd)) = result {
        assert_eq!("", cmd);
    } else {
        panic!("expected unknown instruction error");
    }
    Ok(())
}

#[test]
fn parse_include_missing() -> Result<()> {
    let file = "tests/fixtures/bad-include.sh";
    let result = ScriptFile::parse(file);
    assert!(matches!(result, Err(Error::Include(_, _))));
    Ok(())
}
