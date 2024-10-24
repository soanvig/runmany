use colored::*;
use std::env;
use std::io::{BufRead, BufReader};
use std::process::{Command, ExitCode, Stdio};
use std::thread;

static COLORS: [&str; 5] = ["green", "yellow", "blue", "magenta", "cyan"];

#[derive(Clone)]
struct RunmanyOptions {
    help: bool,
    version: bool,
    no_color: bool,
}

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().collect();

    args.remove(0);
    let parsed_args = parse_args(args);
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

        spawn_commands(commands, &runmany_options);
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
    println!("  --no-color - do not color command output");
}

fn print_version() {
    let version = env!("CARGO_PKG_VERSION");
    println!("v{}", version)
}

fn runmany_args_to_options(args: &Vec<String>) -> RunmanyOptions {
    // todo: wtf is wrong with those types :D
    let help = args.contains(&"-h".to_string()) || args.contains(&"--help".to_string());
    let version = args.contains(&"-v".to_string()) || args.contains(&"--version".to_string());
    let no_color = args.contains(&"--no-color".to_string());

    RunmanyOptions {
        help,
        version,
        no_color,
    }
}

fn spawn_commands(commands: &[Vec<String>], options: &RunmanyOptions) {
    let mut handles = vec![];

    for (index, command) in commands.iter().enumerate() {
        let command = command.clone();
        let options = options.clone();
        let handle = thread::spawn(move || {
            spawn_command(command, index + 1, options);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

/// command_number has to start from 1
fn spawn_command(command_with_args: Vec<String>, command_number: usize, options: RunmanyOptions) {
    let color = COLORS[(command_number - 1) % COLORS.len()];

    let print_color = move |str: String| {
        if options.no_color {
            println!("{}", str);
        } else {
            println!("{}", str.color(color));
        }
    };

    print_color(format!(
        "[{}]: Spawning command: \"{}\"",
        command_number,
        command_with_args.join(" ")
    ));

    let mut child = Command::new(command_with_args.get(0).expect("Command should be defined"))
        .args(&command_with_args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start process");

    let stdout = BufReader::new(child.stdout.take().expect("Cannot reference stdout"));
    let stdout_handle = thread::spawn(move || {
        for line in stdout.lines() {
            print_color(format!(
                "[{}]: {}",
                command_number,
                line.expect("stdout to be line")
            ));
        }
    });

    let stderr = BufReader::new(child.stderr.take().expect("Cannot reference stderr"));
    let stderr_handle = thread::spawn(move || {
        for line in stderr.lines() {
            print_color(format!(
                "[{}]: {}",
                command_number,
                line.expect("stdout to be line")
            ));
        }
    });

    stdout_handle.join().unwrap();
    stderr_handle.join().unwrap();

    let status_code = child.wait().unwrap();

    if status_code.success() {
        print_color(format!(
            "[{}]: Command finished successfully",
            command_number
        ));
    } else {
        print_color(format!(
            "[{}]: Command exited with status: {}",
            command_number,
            status_code
                .code()
                .map(|code| code.to_string())
                .unwrap_or("unknown".to_string())
        ));
    }
}

fn parse_args<'a>(args: Vec<String>) -> Vec<Vec<String>> {
    args.split(|arg| arg == "::")
        .enumerate()
        // Keep first part as possibly empty
        .filter(|(index, part)| *index == 0 || part.len() > 0)
        .map(|(_, part)| part.to_vec())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args_input(vec: Vec<&str>) -> Vec<String> {
        vec.iter().map(|i| i.to_string()).collect()
    }

    #[test]
    fn test_parse_args() {
        let input = parse_args_input(vec![""]);
        let expected: Vec<Vec<String>> = vec![parse_args_input(vec![""])];

        assert_eq!(parse_args(input), expected);
    }
}
