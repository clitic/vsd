use clap::{Arg, Command, CommandFactory};
use std::collections::BTreeMap;
use std::{env, fs, path::Path};

fn main() {
    fs::write(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/cli.md"),
        generate_markdown(&vsd::Args::command()),
    )
    .unwrap();
}

fn generate_markdown(cmd: &Command) -> String {
    let mut buffer = String::new();

    buffer.push_str(&format!(
        "---\nicon: lucide/terminal\n---\n\n\
        # {} CLI\n\n\
        This document contains cli reference for the `vsd` command-line program.\n\n",
        cmd.get_name().to_uppercase(),
    ));

    let mut all_commands = Vec::new();
    collect_commands(cmd, &[], &mut all_commands);

    buffer.push_str("## Command Overview\n\n");
    for cmd_path in &all_commands {
        let anchor = cmd_path.replace(' ', "-");
        buffer.push_str(&format!("- [`{}`↴](#{})\n", cmd_path, anchor));
    }
    buffer.push('\n');
    write_command(&mut buffer, cmd, &[], 2);
    buffer
}

fn collect_commands(cmd: &Command, parents: &[&str], result: &mut Vec<String>) {
    let cmd_path = parents
        .iter()
        .copied()
        .chain(std::iter::once(cmd.get_name()))
        .collect::<Vec<_>>();
    let full_name = cmd_path.join(" ");
    result.push(full_name);

    for sub in cmd.get_subcommands().filter(|s| !s.is_hide_set()) {
        collect_commands(sub, &cmd_path, result);
    }
}

fn write_command(buffer: &mut String, cmd: &Command, parents: &[&str], level: usize) {
    let cmd_path = parents
        .iter()
        .copied()
        .chain(std::iter::once(cmd.get_name()))
        .collect::<Vec<_>>();
    let full_name = cmd_path.join(" ");

    buffer.push_str(&format!("{} `{}`\n\n", "#".repeat(level), full_name));

    if let Some(about) = cmd.get_long_about().or(cmd.get_about()) {
        buffer.push_str(&format!("{}\n\n", about));
    }

    buffer.push_str("```\n");
    buffer.push_str(&format!("{} [OPTIONS]", full_name));

    let positionals = cmd.get_positionals().collect::<Vec<_>>();
    for arg in &positionals {
        if arg.is_required_set() {
            buffer.push_str(&format!(" <{}>", arg.get_id().to_string().to_uppercase()));
        } else {
            buffer.push_str(&format!(" [{}]", arg.get_id().to_string().to_uppercase()));
        }
    }

    if cmd.has_subcommands() {
        buffer.push_str(" <COMMAND>");
    }
    buffer.push_str("\n```\n\n");

    if !positionals.is_empty() {
        buffer.push_str("**Arguments:**\n\n");
        for arg in &positionals {
            write_arg(buffer, arg);
        }
        buffer.push('\n');
    }

    let subcommands = cmd
        .get_subcommands()
        .filter(|s| !s.is_hide_set())
        .collect::<Vec<_>>();

    if !subcommands.is_empty() {
        buffer.push_str("**Subcommands:**\n\n");
        buffer.push_str("| Command | Description |\n");
        buffer.push_str("|---------|-------------|\n");
        for sub in &subcommands {
            let about = sub.get_about().map(|s| s.to_string()).unwrap_or_default();
            buffer.push_str(&format!("| `{}` | {} |\n", sub.get_name(), about));
        }
        buffer.push('\n');
    }

    let options: Vec<_> = cmd
        .get_arguments()
        .filter(|a| !a.is_positional() && !a.is_hide_set())
        .collect();

    if !options.is_empty() {
        let mut grouped: BTreeMap<Option<String>, Vec<&Arg>> = BTreeMap::new();
        for arg in &options {
            let heading = arg.get_help_heading().map(|s| s.to_string());
            grouped.entry(heading).or_default().push(arg);
        }

        for (heading, args) in grouped {
            let heading_str = heading.as_deref().unwrap_or("Options");
            buffer.push_str(&format!("**{}:**\n\n", heading_str));
            buffer.push_str("| Flag | Description |\n");
            buffer.push_str("|------|-------------|\n");

            for arg in args {
                write_option(buffer, arg);
            }
            buffer.push('\n');
        }
    }

    buffer.push_str("[↑ Back to top](#command-overview)\n\n");

    for sub in subcommands {
        write_command(buffer, sub, &cmd_path, level + 1);
    }
}

fn write_arg(buffer: &mut String, arg: &Arg) {
    let required = if arg.is_required_set() {
        " *(required)*"
    } else {
        ""
    };
    let help = arg
        .get_long_help()
        .or(arg.get_help())
        .map(|s| s.to_string())
        .unwrap_or_default();

    buffer.push_str(&format!(
        "- `<{}>`: {}{}\n",
        arg.get_id().to_string().to_uppercase(),
        help,
        required
    ));
}

fn write_option(buffer: &mut String, arg: &Arg) {
    let mut flags = Vec::new();
    if let Some(short) = arg.get_short() {
        flags.push(format!("-{}", short));
    }
    if let Some(long) = arg.get_long() {
        flags.push(format!("--{}", long));
    }
    let flag_str = flags.join(", ");

    let mut help = arg
        .get_long_help()
        .or(arg.get_help())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let possible_values: Vec<_> = arg.get_possible_values();
    let is_bool = possible_values.len() == 2
        && possible_values
            .iter()
            .all(|v| v.get_name() == "true" || v.get_name() == "false");
    if !possible_values.is_empty() && !is_bool {
        let values: Vec<_> = possible_values.iter().map(|v| v.get_name()).collect();
        help.push_str(&format!("<br>*Possible values:* `{}`", values.join("`, `")));
    }

    if let Some(default) = arg.get_default_values().first() {
        if !arg.is_hide_default_value_set() {
            help.push_str(&format!("<br>*Default:* `{}`", default.to_string_lossy()));
        }
    }

    let help = help.replace('|', "\\|").replace('\n', "<br>");
    buffer.push_str(&format!("| `{}` | {} |\n", flag_str, help));
}
