use std::env;
use std::io::{BufRead, BufReader};
use std::process::{Command, ExitCode, Stdio};
use std::thread;

struct RunmanyOptions {
    help: bool,
    version: bool,
}

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().collect();

    let parsed_args = parse_args(&mut args);
    if let Some((runmany_params, commands)) = parsed_args.split_first() {
        let runmany_options = runmany_args_to_options(runmany_params);

        if runmany_options.help {
            print_help();
            return ExitCode::SUCCESS;
        }

        if runmany_options.version {
            print_version();
            return ExitCode::SUCCESS;
        }

        spawn_commands(commands);
    } else {
        // No arguments given to runmany
        print_help();
    }

    return ExitCode::SUCCESS;
}

fn print_help() {
    let version = env!("CARGO_PKG_VERSION");
    println!("runmany - v{}", version);
    println!("Easily run multiple long-running commands in parallel.");
    println!("");
    println!("Usage: runmany [RUNMANY FLAGS] [:: <COMMAND>] [:: <COMMAND>] [:: <COMMAND>]");
    println!("");
    println!("Flags:");
    println!("  -h, --help - print help");
    println!("  -v, --version - print version");
}

fn print_version() {
    let version = env!("CARGO_PKG_VERSION");
    println!("v{}", version)
}

fn runmany_args_to_options(args: &&[String]) -> RunmanyOptions {
    // todo: wtf is wrong with those types :D
    let help = args.contains(&"-h".to_string()) || args.contains(&"--help".to_string());
    let version = args.contains(&"-v".to_string()) || args.contains(&"--version".to_string());

    RunmanyOptions { help, version }
}

fn spawn_commands(commands: &[&[String]]) {
    let mut handles = vec![];

    for (index, &command) in commands.iter().enumerate() {
        let command = command.to_vec();
        let handle = thread::spawn(move || {
            spawn_command(command, index + 1);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn spawn_command(command_with_args: Vec<String>, command_number: usize) {
    println!(
        "[{}]: Spawning command: \"{}\"",
        command_number,
        command_with_args.join(" ")
    );

    let mut child = Command::new(command_with_args.get(0).expect("Command should be defined"))
        .args(&command_with_args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start process");

    let stdout = BufReader::new(child.stdout.take().expect("Cannot reference stdout"));
    let stdout_handle = thread::spawn(move || {
        for line in stdout.lines() {
            println!("[{}]: {}", command_number, line.expect("stdout to be line"));
        }
    });

    let stderr = BufReader::new(child.stderr.take().expect("Cannot reference stderr"));
    let stderr_handle = thread::spawn(move || {
        for line in stderr.lines() {
            println!("[{}]: {}", command_number, line.expect("stdout to be line"));
        }
    });

    stdout_handle.join().unwrap();
    stderr_handle.join().unwrap();

    let status_code = child.wait().unwrap();

    if status_code.success() {
        println!("[{}]: Command finished successfully", command_number)
    } else {
        println!(
            "[{}]: Command exited with status: {}",
            command_number,
            status_code
                .code()
                .map(|code| code.to_string())
                .unwrap_or("unknown".to_string())
        )
    }
}

fn parse_args<'a>(args: &'a mut Vec<String>) -> Vec<&'a [String]> {
    args.remove(0);

    args.split(|arg| arg == "::")
        .enumerate()
        // Keep first part as possibly empty
        .filter(|(index, part)| *index == 0 || part.len() > 0)
        .map(|(_, part)| part)
        .collect()
}
