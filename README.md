# Anticipate

Script based automation for [rexpect](https://docs.rs/rexpect/latest/rexpect/) with support for [asciinema][].

Perfect for automating demos of CLI tools.

## Install

```
cargo install anticipate
```

## Usage

To record using [asciinema][] writing a `.cast` file for each input file into the `target` directory overwriting any existing files:

```
anticipate \
  record \
  --overwrite \
  --logs target \
  target \
  test/fixtures/*.sh
```

To finish recording we send the `exit` command which will be captured and included in the recording. For demos there is no need to show the exit command so we trim the resulting file to remove it by default. If you want to keep those lines in the recording then set `--trim-lines 0`.

## License

MIT or Apache-2.0

[asciinema]: https://asciinema.org/
