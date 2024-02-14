use anticipate_core::ScriptFile;
use anyhow::Result;

#[test]
fn interpret_echo() -> Result<()> {
    let mut files =
        ScriptFile::parse_files(vec!["tests/fixtures/echo.sh".into()])?;
    let file = files.remove(0);
    file.run(Default::default());
    Ok(())
}

#[test]
fn interpret_teletype() -> Result<()> {
    let mut files =
        ScriptFile::parse_files(vec!["tests/fixtures/teletype.sh".into()])?;
    let file = files.remove(0);
    file.run(Default::default());
    Ok(())
}

#[test]
fn interpret_env_var() -> Result<()> {
    std::env::set_var("MOCK_PASSWORD", "foobar");
    let mut files = ScriptFile::parse_files(vec![
        "tests/fixtures/password-env.sh".into(),
    ])?;
    let file = files.remove(0);
    file.run(Default::default());
    Ok(())
}
