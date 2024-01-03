<div align="center">

![gate](../resources/logo.png)

gate is currently in alpha

A cross-platform TUI database management tool written in Rust

[![github workflow status](https://img.shields.io/github/workflow/status/TaKO8Ki/gate/CI/main)](https://github.com/TaKO8Ki/gate/actions) 
<!-- [![crates](https://img.shields.io/crates/v/gate.svg?logo=rust)](https://crates.io/crates/gate) -->

![gate](../resources/gate.gif)

</div>

## Features

- Cross-platform support (macOS, Windows, Linux)
- Multiple Database support (MySQL, PostgreSQL, MSSQL, SQLite)
- Intuitive keyboard only control

## TODOs

- [ ] Full Support to MSSQL
- [ ] SQL editor
- [ ] Custom key bindings
- [ ] Custom theme settings
- [ ] Support other databases

## Usage

```
$ gate
```

```
$ gate -h
USAGE:
    gate [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config-path <config-path>    Set the config file
```

If you want to add connections, you need to edit your config file. For more information, please see [Configuration](#Configuration).

## Keymap

| Key | Description |
| ---- | ---- |
| <kbd>h</kbd>, <kbd>j</kbd>, <kbd>k</kbd>, <kbd>l</kbd> | Scroll left/down/up/right |
| <kbd>Ctrl</kbd> + <kbd>u</kbd>, <kbd>Ctrl</kbd> + <kbd>d</kbd> | Scroll up/down multiple lines |
| <kbd>g</kbd> , <kbd>G</kbd> | Scroll to top/bottom |
| <kbd>H</kbd>, <kbd>J</kbd>, <kbd>K</kbd>, <kbd>L</kbd> | Extend selection by one cell left/down/up/right |
| <kbd>y</kbd> | Copy a cell value |
| <kbd>←</kbd>, <kbd>→</kbd> | Move focus to left/right |
| <kbd>c</kbd> | Move focus to connections |
| <kbd>/</kbd> | Filter |
| <kbd>?</kbd> | Help |
| <kbd>1</kbd>, <kbd>2</kbd>, <kbd>3</kbd>, <kbd>4</kbd>, <kbd>5</kbd> | Switch to records/columns/constraints/foreign keys/indexes tab |
| <kbd>Esc</kbd> | Hide pop up |

## Configuration

The location of the file depends on your OS:

- macOS: `$HOME/.config/gate/config.toml`
- Linux: `$HOME/.config/gate/config.toml`
- Windows: `%APPDATA%/gate/config.toml`

The following is a sample config.toml file:

```toml
[[conn]]
type = "mysql"
host = "localhost"
port = 3306
user = "root"
password = "password"
database = "foo"
name = "mysql Foo DB"

[[conn]]
type = "postgres"
user = "root"
host = "localhost"
port = 5432
database = "bar"
name = "postgres Bar DB"

[[conn]]
type = "mssql"
user = "root"
host = "localhost"
port = 1433
database = "bar"
name = "mssql Foo DB"

[[conn]]
type = "sqlite"
path = "/path/to/bar.db"
```

## Contribution

Contributions, issues and pull requests are welcome!
