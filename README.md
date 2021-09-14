# process-queue 🚌

[![Build status](https://github.com/Soft/process-queue/workflows/ci/badge.svg)](https://github.com/Soft/process-queue/actions)
[![Latest Version](https://img.shields.io/crates/v/process-queue.svg)](https://crates.io/crates/process-queue)
[![GitHub release](https://img.shields.io/github/release/Soft/process-queue.svg)](https://github.com/Soft/process-queue/releases)
[![dependency status](https://deps.rs/repo/github/soft/process-queue/status.svg)](https://deps.rs/repo/github/soft/process-queue)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`pqueue` is a command-line task queue.

Multiple queues can be created and each queue can have its own configuration.

## Installation

Statically linked release binaries are available on [GitHub
releases](https://github.com/Soft/process-queue/releases). These should work on
any modern x86-64 Linux system.

Alternatively, `pqueue` can be installed using `cargo`:

```
cargo install process-queue
```

## Getting Started

All of the following examples assume that `pqueue` server has been first started
with

```
pqueue start
```

This starts `pqueue` server in the background. If desired, `--foreground` (`-f`)
flag can be specified to keep the server attached to the terminal.

The simplest possible way to use `pqueue` is to create the default queue using
the default settings:

```
pqueue create
```

This creates a default task queue that sequentially executes each submitted
task. We can submit tasks for execution using `send` sub-command:

```
pqueue send echo "hello world"
pqueue send true
pqueue send sleep 60
pqueue send curl example.com
```

This queued four tasks for execution starting with `echo`. List of the pending
tasks in a queue can be inspected using `tasks` sub-command.

Multiple queues can be created by supplying queue name using the `--name` (`-n`)
option when creating the queue. If no name is given `pqueue` sub-commands
implicitly operate on a queue named `default`.

`pqueue` can be used for queueing time consuming tasks for execution. For
example, we might use `pqueue` for queueing file downloads. The following will
create a task queue that sequentially executes `wget` with each queued URL as an
argument.

```
pqueue create -n downloads -t "wget {}"
pqueue send -n downloads example.com
pqueue send -n downloads example.org
```

This create a new task queue named `downloads` with a task template that species
that each new task sent to the queue should be interpreted as an argument to
`wget`.

See `Task Templates` section for more information regarding queue templates.

By default, `pqueue` executes each task sequentially. This can be changed by
specifying `--max-parallel` (`-p`) option when creating the task queue. For
example, the following command can be used create a queue that executes up to
three tasks in parallel.

```
pqueue create -n sleepers -p 3 -t "sleep {}"
pqueue send -n sleepers 60
pqueue send -n sleepers 120
pqueue send -n sleepers 180
pqueue send -n sleepers 240
```

This will create a queue named `sleepers` for invoking `sleep` command with
different arguments. Four tasks are submitted to the queue, three of which will
begin executing immediately while the fourth task remains in the queue until
free execution slots become available.

## Usage

### `pqueue`

```
Task queue

USAGE:
    pqueue [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -s, --socket <socket>    Server socket path

SUBCOMMANDS:
    start-server    Start queue server [aliases: start]
    stop-server     Stop queue server [aliases: stop]
    create-queue    Create new task queue [aliases: create]
    remove-queue    Remove task queue [aliases: remove]
    list-queues     List queues [aliases: queues]
    send-task       Send task to a queue [aliases: send]
    list-tasks      List tasks in a queue [aliases: tasks]
    help            Prints this message or the help of the given subcommand(s)
```

### `pqueue start-server`

```
Start queue server

USAGE:
    pqueue start-server [FLAGS] [OPTIONS]

FLAGS:
    -f, --foreground    Keep pqueue server in the foreground
    -h, --help          Prints help information
    -v                  Log level
    -V, --version       Prints version information

OPTIONS:
    -l, --log-file <log-file>    Log file
```

### `pqueue stop-server`

```
Stop queue server

USAGE:
    pqueue stop-server

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

### `pqueue create-queue`

```
Create new task queue

USAGE:
    pqueue create-queue [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -s, --stdout     Output to stdin
    -V, --version    Prints version information

OPTIONS:
    -d, --dir <dir>                      Default working directory
    -f, --file <file>                    Output to file
    -p, --max-parallel <max-parallel>    Maximum number of parallel tasks [default: 1]
    -n, --name <name>                    Queue name [default: default]
    -t, --template <template>            Task template
    -T, --timeout <timeout>              Default task timeout
```

### `pqueue remove-queue`

```
Remove task queue

USAGE:
    pqueue remove-queue [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -n, --name <name>    Queue name [default: default]
```

### `pqueue list-queues`

```
List queues

USAGE:
    pqueue list-queues

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

### `pqueue send-task`

```
Send task to a queue

USAGE:
    pqueue send-task [OPTIONS] [args]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --dir <dir>            Working directory
    -n, --name <name>          Task name [default: default]
    -T, --timeout <timeout>    Task timeout

ARGS:
    <args>...
```

### `pqueue list-tasks`

```
List tasks in a queue

USAGE:
    pqueue list-tasks [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -n, --name <name>    Task name [default: default]
```

## Task Templates

By default queues can execute arbitrary commands. It is however possible to make
specialized task queues that have a task template associated with them. When a
new task is sent to a queue that has a task template associated with it, the
template is expanded using the arguments supplied to `send-task`. When a queue
is created, a task template can be specified using the `--template` option.

Task templates specify the command that will be executed. The template can
contain zero or more `{}` placeholders that will be replaced with the arguments
supplied to `send-task`.

Templates can also contain at most one `{...}` placeholder. This placeholders
accepts variable number of arguments.

## Issues

Bugs should be reported at [GitHub](https://github.com/Soft/process-queue/issues).
