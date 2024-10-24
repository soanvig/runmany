use colored::*;
use std::env;
use std::io::{stdout, BufRead, BufReader, Write};
use std::process::{Command, ExitCode, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

static COLORS: [&str; 5] = ["green", "yellow", "blue", "magenta", "cyan"];

#[derive(Clone, PartialEq, Debug)]
struct RunmanyOptions {
    help: bool,
    version: bool,
    no_color: bool,
}

#[derive(Clone, PartialEq, Debug)]
struct Printer<W: Write> {
    options: RunmanyOptions,
    writer: W,
    prefix: Option<String>,
    color: Option<String>,
}

impl<W: Write> Printer<W> {
    fn new(options: RunmanyOptions, writer: W) -> Printer<W> {
        Printer {
            options,
            writer,
            prefix: None,
            color: None,
        }
    }

    fn set_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);

        self
    }

    fn set_color(mut self, color: String) -> Self {
        self.color = Some(color);

        self
    }

    fn print(&mut self, str: String) {
        let to_print = {
            if self.options.no_color {
                str
            } else if let Some(color) = &self.color {
                str.color(color.to_owned()).to_string()
            } else {
                str
            }
        };

        self.writer.write_all(to_print.as_bytes()).unwrap();
        self.writer.write_all(b"\n").unwrap();
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    run(args)
}

fn run(mut args: Vec<String>) -> ExitCode {
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
    println!("Example: runmany :: npm build:watch :: npm serve");
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
        let printer = Printer::new(options, stdout())
            .set_prefix(format!("[{}]", index + 1))
            .set_color(COLORS[(index) % COLORS.len()].to_string());

        let handle = thread::spawn(move || {
            spawn_command(command, index + 1, printer);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

/// command_number has to start from 1
///
/// command's stderr is logged to stdout
///
/// todo: might need a refactor due to Arc<Mutex>> that requires locking. Maybe there is simple way to do it
fn spawn_command<W: Write + Send + 'static>(
    command_with_args: Vec<String>,
    command_number: usize,
    mut printer: Printer<W>,
) {
    printer.print(format!(
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

    let printer_arc = Arc::new(Mutex::new(printer));
    let main_printer = printer_arc.clone();

    let stdout = BufReader::new(child.stdout.take().expect("Cannot reference stdout"));
    let stdout_printer = printer_arc.clone();
    let stdout_handle = thread::spawn(move || {
        for line in stdout.lines() {
            stdout_printer.lock().unwrap().print(format!(
                "[{}]: {}",
                command_number,
                line.expect("stdout to be line")
            ));
        }
    });

    let stderr = BufReader::new(child.stderr.take().expect("Cannot reference stderr"));
    let stderr_printer = printer_arc.clone();
    let stderr_handle = thread::spawn(move || {
        for line in stderr.lines() {
            stderr_printer.lock().unwrap().print(format!(
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
        main_printer.lock().unwrap().print(format!(
            "[{}]: Command finished successfully",
            command_number
        ));
    } else {
        main_printer.lock().unwrap().print(format!(
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

    fn to_vec_str(vec: Vec<&str>) -> Vec<String> {
        vec.iter().map(|i| i.to_string()).collect()
    }

    #[test]
    fn test_parse_args() {
        let input = to_vec_str(vec![""]);
        let expected: Vec<Vec<String>> = vec![to_vec_str(vec![""])];
        assert_eq!(parse_args(input), expected);

        let input = to_vec_str(vec!["-v"]);
        let expected: Vec<Vec<String>> = vec![to_vec_str(vec!["-v"])];
        assert_eq!(parse_args(input), expected);

        let input = to_vec_str(vec!["-v", "-r"]);
        let expected: Vec<Vec<String>> = vec![to_vec_str(vec!["-v", "-r"])];
        assert_eq!(parse_args(input), expected);

        let input = to_vec_str(vec!["-v", "-r", "::"]);
        let expected: Vec<Vec<String>> = vec![to_vec_str(vec!["-v", "-r"])];
        assert_eq!(parse_args(input), expected);

        let input = to_vec_str(vec!["-v", "-r", "::", "command"]);
        let expected: Vec<Vec<String>> =
            vec![to_vec_str(vec!["-v", "-r"]), to_vec_str(vec!["command"])];
        assert_eq!(parse_args(input), expected);

        let input = to_vec_str(vec!["-v", "-r", "::", "command", "-v"]);
        let expected: Vec<Vec<String>> = vec![
            to_vec_str(vec!["-v", "-r"]),
            to_vec_str(vec!["command", "-v"]),
        ];
        assert_eq!(parse_args(input), expected);

        let input = to_vec_str(vec!["-v", "-r", "::", "command", "-v", "::"]);
        let expected: Vec<Vec<String>> = vec![
            to_vec_str(vec!["-v", "-r"]),
            to_vec_str(vec!["command", "-v"]),
        ];
        assert_eq!(parse_args(input), expected);

        let input = to_vec_str(vec!["-v", "-r", "::", "command", "-v", "::", "command2"]);
        let expected: Vec<Vec<String>> = vec![
            to_vec_str(vec!["-v", "-r"]),
            to_vec_str(vec!["command", "-v"]),
            to_vec_str(vec!["command2"]),
        ];
        assert_eq!(parse_args(input), expected);

        let input = to_vec_str(vec!["-v", "-r", "::", "command::xxx", "-v"]);
        let expected: Vec<Vec<String>> = vec![
            to_vec_str(vec!["-v", "-r"]),
            to_vec_str(vec!["command::xxx", "-v"]),
        ];
        assert_eq!(parse_args(input), expected);
    }

    #[test]
    fn test_runmany_args_to_options() {
        let input = to_vec_str(vec!["-v"]);
        let expected = RunmanyOptions {
            help: false,
            no_color: false,
            version: true,
        };
        assert_eq!(runmany_args_to_options(&input), expected);

        let input = to_vec_str(vec!["-h"]);
        let expected = RunmanyOptions {
            help: true,
            no_color: false,
            version: false,
        };
        assert_eq!(runmany_args_to_options(&input), expected);

        let input = to_vec_str(vec!["--no-color"]);
        let expected = RunmanyOptions {
            help: false,
            no_color: true,
            version: false,
        };
        assert_eq!(runmany_args_to_options(&input), expected);

        let input = to_vec_str(vec!["-v", "-h", "--no-color"]);
        let expected = RunmanyOptions {
            help: true,
            no_color: true,
            version: true,
        };
        assert_eq!(runmany_args_to_options(&input), expected);

        let input = to_vec_str(vec!["--not-existing", "-n"]);
        let expected = RunmanyOptions {
            help: false,
            no_color: false,
            version: false,
        };
        assert_eq!(runmany_args_to_options(&input), expected);
    }
}
