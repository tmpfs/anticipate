# Anticipate

Script based automation using [rexpect](https://docs.rs/rexpect/latest/rexpect/) with support for [asciinema][].

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

See the progam help for more options.

## Syntax

* [pragma](#pragma) - `#!`
* [sendline](#send-line) - `#$ sendline ls -la`
* [sendcontrol](#send-control) - `#$ sendcontrol c`
* [expect](#expect) - `#$ expect Documents`
* [regex](#regex) - `#$ regex [0-9]`
* [readline](#read-line) - `#$ readline`
* [wait](#wait) - `#$ wait 500`

The syntax is inspired by [asciinema-automation](https://github.com/PierreMarchand20/asciinema_automation/).

### Pragma

Use a pragma as the first instruction to set the command to execute:

```
#!sh
```

If a relative path is given it is resolved relative to the script:

```
#!../programs/script.sh
```

### Send Line

Raw text is sent as a line to the pseudo-terminal:

```
ls -la
```

Or you can use the sendline command explicitly:

```
#$ sendline ls -la
```

### Send Control

To send a control character, for example Ctrl+C:

```
#$ sendcontrol c
```

### Expect

Expect waits for a string to appear in the program output:

```
#$ expect Documents
```

### Regex

To wait for a pattern to appear in the program output use `regex`:

```
#$ regex [0-9]
```

### Read Line

Read a line of program output:

```
#$ readline
```

### Wait

To wait for a number of milliseconds:

```
#$ wait 500
```

## License

MIT or Apache-2.0

[asciinema]: https://asciinema.org/
