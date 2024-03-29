use anticipate_runner::{InterpreterOptions, ScriptFile};
use anyhow::Result;

#[test]
fn interpret_echo() -> Result<()> {
    let file = ScriptFile::parse("tests/fixtures/echo.sh")?;
    file.run(Default::default())?;
    Ok(())
}

#[test]
fn interpret_escaped_newline() -> Result<()> {
    let file = ScriptFile::parse("tests/fixtures/escaped-newline.sh")?;
    file.run(Default::default())?;
    Ok(())
}

#[test]
fn interpret_teletype() -> Result<()> {
    let file = ScriptFile::parse("tests/fixtures/teletype.sh")?;
    file.run(Default::default())?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn interpret_env_var() -> Result<()> {
    std::env::set_var("MOCK_PASSWORD", "foobar");
    let file = ScriptFile::parse("tests/fixtures/password-env.sh")?;
    let options: InterpreterOptions = Default::default();
    file.run(options)?;
    Ok(())
}

#[cfg(windows)]
#[test]
fn interpret_env_var() -> Result<()> {
    std::env::set_var("MOCK_PASSWORD", "foobar");
    let file = ScriptFile::parse("tests/fixtures/password-env-windows.sh")?;
    let options: InterpreterOptions = Default::default();
    file.run(options)?;
    Ok(())
}

#[test]
fn interpret_include() -> Result<()> {
    let file = ScriptFile::parse("tests/fixtures/include.sh")?;
    file.run(Default::default())?;
    Ok(())
}
