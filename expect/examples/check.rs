use expectrl::{check, spawn, Error};

#[cfg(not(feature = "async"))]
fn main() {
    let mut session = spawn("python ./tests/source/ansi.py").expect("Can't spawn a session");

    loop {
        match check!(
            &mut session,
            _ = "Password: " => {
                println!("Set password to SECURE_PASSWORD");
                session.send_line("SECURE_PASSWORD").unwrap();
            },
            _ = "Continue [y/n]:" => {
                println!("Stop processing");
                session.send_line("n").unwrap();
            },
        ) {
            Err(Error::Eof) => break,
            result => result.unwrap(),
        };
    }
}

#[cfg(feature = "async")]
fn main() {
    futures_lite::future::block_on(async {
        let mut session = spawn("python ./tests/source/ansi.py").expect("Can't spawn a session");

        loop {
            match check!(
                &mut session,
                _ = "Password: " => {
                    println!("Set password to SECURE_PASSWORD");
                    session.send_line("SECURE_PASSWORD").await.unwrap();
                },
                _ = "Continue [y/n]:" => {
                    println!("Stop processing");
                    session.send_line("n").await.unwrap();
                },
            )
            .await
            {
                Err(Error::Eof) => break,
                result => result.unwrap(),
            };
        }
    })
}
