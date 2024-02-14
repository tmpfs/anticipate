use anticipate_core::ScriptParser;
use anyhow::Result;

#[test]
fn parse_pragma() -> Result<()> {
    let source = r#"#!../programs/foo.sh"#;
    let instructions = ScriptParser.parse(source)?;
    println!("{:#?}", commands);
    assert_eq!(1, instructions.len());
    Ok(())
}

#[test]
fn parse_sendline() -> Result<()> {
    let source = r#"
#$ sendline foo
#$ expect bar
#$ regex [0-9]
#$ sendcontrol c
"#;
    let commands = ScriptParser.parse(source)?;
    println!("{:#?}", commands);
    Ok(())
}
