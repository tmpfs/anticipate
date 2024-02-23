# Anticipate

Script based automation like `expect(1)` with support for [asciinema][].

Perfect for demos and testing command line interfaces.

## Install

```
cargo install anticipate-cli
```

## Example

Create a directory, initialize a server config, print the config file, start the server and then send SIGTERM to shut it down.

```shell
mkdir -p target/server/accounts
#$ readline

server init target/config.toml --path server/accounts
#$ readline

cat target/config.toml
#$ expect path = "server/accounts"

server start target/config.toml
#$ sendcontrol ^C
```

See the `examples` and `fixtures` folders for more examples.

## Usage

### Running

To execute scripts use the `run` command:

```
anticipate \
  run \
  --parallel \
  --log \
  tests/examples/*.sh
```

### Recording

To record using [asciinema][] writing a `.cast` file for each input file into the `target` directory overwriting any existing files:

```
anticipate \
  record \
  --parallel \
  --overwrite \
  --log \
  target \
  tests/examples/*.sh
```

See the progam help for more options.

## Syntax

* [pragma](#pragma) - `#!/bin/bash`
* [sendline](#send-line) - `#$ sendline ls -la`
* [sendcontrol](#send-control) - `#$ sendcontrol ^C`
* [expect](#expect) - `#$ expect Documents`
* [regex](#regex) - `#$ regex [0-9]`
* [readline](#read-line) - `#$ readline`
* [sleep](#sleep) - `#$ sleep 500`
* [send](#send) - `#$ send echo`
* [flush](#flush) - `#$ flush`
* [wait](#wait) - `#$ wait`
* [clear](#clear) - `#$ clear`
* [include](#include) - `#$ include ../shared.sh`

Environment variables are interpolated for commands sent to the pseudo terminal which makes it easier to share values across scripts. 

```
export NAME=foo
anticipate rec -o target tests/examples/interpolate.sh
asciinema play target/interpolate.cast
```

### Pragma

Use a pragma as the first instruction to set the command to execute:

```
#!/bin/bash
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
#$ sendcontrol ^C
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

### Sleep

Wait for a number of milliseconds:

```
#$ sleep 500
```

### Send

Send text to the program without flushing the stream:

```
#$ send echo
```

### Flush

Flush the buffer being sent to the pseudo-terminal:

```
#$ flush
```

### Wait

Wait for the prompt to appear:

```
#$ wait
```

### Clear

Clear the screen and reset the cursor position:

```
#$ clear
```

### Include

Include instructions from a script file:

```
#$ include ../shared.sh
```

Paths are resolved relative to the parent directory of the script file.

## See Also

* [Autocast](https://github.com/k9withabone/autocast) if you prefer a YAML syntax
* [Asciinema Integrations](https://docs.asciinema.org/integrations/) for other asciinema tools 

## Credits

The [core](/core) module is a fork of [expectrl](https://github.com/zhiburt/expectrl/) and the syntax is inspired by [asciinema-automation](https://github.com/PierreMarchand20/asciinema_automation/).

## License

MIT or Apache-2.0

[asciinema]: https://asciinema.org/
