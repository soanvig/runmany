# runmany

Easily run multiple long-running commands in parallel.

No more spawning processes in shell's background (`&`) or starting multiple terminals just to run few commands.

## Usage

```sh
# No troubling control characters like "<command>"
# Just use double colon to separate all commands
> runmany :: npm watch :: npm serve

# You can run more commands
> runmany :: npm watch :: npm serve :: npm test:watch
```

Now `runmany` will run all commands in parallel, and exit when all exit.

## Installation

Runmany is currently available only for Linux.

If you already have a Rust environment set up, you can use the cargo install command:

```sh
> cargo install runmany
```

@TODO: add releases with built binaries