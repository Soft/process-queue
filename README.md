# process-queue :train:

process-queue is a tool for queuing sequential program executions using
different sets of arguments.

## Examples

process-queue server can be started with the `server` sub-command. In this
example, we start a pqueue server that will execute `echo` with two arguments,
"Hello" and one supplied by the sender.

    pqueue server echo Hello {}

Once the server is listening we can start sending in jobs by running:

    pqueue send world

We can see the string "Hello world" got printed in the terminal running the
server. The server will keep listening for new jobs, if we now execute:

    pqueue send "John Doe"

We will see "Hello John Doe" printed as expected. Where `pqueue` comes in handy
is when dealing with long running jobs. Since the jobs are queued one can send
in new a job even if the server is still executing an earlier program. All the
jobs are stored in a queue and executed sequentially in first-come, first-served
basis. For example, we can start a new server named "timers":

    pqueue server -n timers sleep {}

After this, we can send in a bunch of jobs:

    pqueue send -n timers 10
    pqueue send -n timers 4
    pqueue send -n timers 20

The tasks will execute one after another.

## Dependencies

process-queue uses DBus for IPC and
needs [`libdbus`](https://dbus.freedesktop.org/releases/dbus/) 1.6 or higher.
