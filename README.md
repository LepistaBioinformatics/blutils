# BLUTILS

`Blutils` (Blast Utils) is a BlastN wrapper developed to execute with hight
performance on improve the native Blast parallelism. The main feature of
`Blutils` is to allow users to generate consensus identities from multi-identity
blast outputs.

## Installation

`Blutils` package could be installed directly from
[crates.io](https://crates.io/crates/blutils-cli) using cargo:

```bash
cargo install blutils-cli
```

After installed, `Blutils` you should evoke it using the `blu` command.

```bash
blu --help
```

The output should be close to:

```bash
The CLI port of the blutils library

Usage: blu [OPTIONS] <COMMAND>

Commands:
  build-db  Build the blast database as a pre-requisite for the blastn command
  blastn    Execute the parallel blast and run consensus algorithm
  check     Check `Blutils` dependencies
  help      Print this message or the help of the given subcommand(s)

Options:
      --log-level <LOG_LEVEL>

      --log-file <LOG_FILE>

      --log-format <LOG_FORMAT>
          [default: ansi]

          Possible values:
          - ansi:  ANSI format
          - jsonl: YAML format

  -t, --threads <THREADS>
          [default: 1]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Check dependencies

Blutils depends of `Ncbi-Blast+` to be installed on the host system. To check if
the host OS has these package installed run the `Blutils` checker for linux
systems:

```bash
blu check linux
```

Note: Currently the system check is available only for linux systems and assumes
that dependencies could be evoked directly from terminal.

## Read the Book

The `Blutils` book is a comprehensive guide to the `Blutils` package. It is
available in [Blutils Book in
GitHub](https://github.com/LepistaBioinformatics/blutils/blob/main/docs/book/README.md).
