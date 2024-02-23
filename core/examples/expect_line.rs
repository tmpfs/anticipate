use anticipate::{self, Any, Eof};

fn main() {
    let mut session =
        anticipate::spawn("ls -al").expect("Can't spawn a session");

    loop {
        let m = session
            .expect(Any::boxed(vec![
                Box::new("\r"),
                Box::new("\n"),
                Box::new(Eof),
            ]))
            .expect("Expect failed");

        println!("{:?}", String::from_utf8_lossy(m.as_bytes()));

        let is_eof = m[0].is_empty();
        if is_eof {
            break;
        }

        if m[0] == [b'\n'] {
            continue;
        }

        println!("{:?}", String::from_utf8_lossy(&m[0]));
    }
}
