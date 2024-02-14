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

[asciinema]: https://asciinema.org/
