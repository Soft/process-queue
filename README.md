# process-queue :train:

process-queue is a tool for queuing sequential program executions using
different sets of arguments. It can be useful for managing long-running tasks.

## Installation

    cargo install --git 'https://bitbucket.org/Soft/process-queue.git'

## Usage

    pqueue server [-h|--help] [-V|--version] [-n|--name NAME] [-c|--cd DIR] [-d|--daemon] [-r|--retries N] COMMAND TEMPLATE...

    pqueue send [-h|--help] [-V|--version] [-n|--name NAME] ARGS...

    pqueue stop [-h|--help] [-V|--version] [-n|--name NAME]

    pqueue has [-h|--help] [-V|--version] [-n|--name NAME]

## Templates

When a `pqueue` server is started, a program argument template can be specified.
Template is a list of program arguments and one or more "placeholders", that can
be filled in by the sender. In the argument templates, `{}` specifies a
placeholder. The placeholders will be replaced with the arguments specified with
the `send` sub-command.

## Examples

The `pqueue` server can be started with the `server` sub-command. In this
example, we start a `pqueue` server that will execute `echo` with two arguments,
"Hello" and one supplied by the sender.

    pqueue server echo Hello {}

Once the server is listening, we can start sending in tasks. For example, let's
greet the world by running:

    pqueue send world

We can see that the string "Hello world" got printed in the terminal where the
server is running. The server will keep listening for new tasks, if we now
execute:

    pqueue send "John Doe"

We will see "Hello John Doe" printed as expected.

Where `pqueue` comes in handy is when dealing with long running tasks. Since the
tasks are queued one can send in new a task even if the server is still
executing an earlier program. All the tasks are stored in a queue and executed
sequentially on a first-come, first-served basis.

To demonstrate this, we can start a new server named "timers":

    pqueue server -n timers sleep {}

After this, we can send in a bunch of tasks:

    pqueue send -n timers 10
    pqueue send -n timers 4
    pqueue send -n timers 20

The tasks will execute one after another and new ones can be added even if the
old ones are not finished.

A server can be stopped with the `pqueue stop` command. If a server receives a
stop request while it is processing a task, it will first wait for the current
task to finish before stopping. When a stop request is received, any queued
tasks will be discarded.

## Dependencies

process-queue uses DBus for IPC and
needs [`libdbus`](https://dbus.freedesktop.org/releases/dbus/) 1.6 or higher.
