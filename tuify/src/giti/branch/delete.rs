use self::giti_ui_templates::ask_user_to_select_from_list;
use crate::*;
use r3bl_ansi_color::{AnsiStyledText, Color, Style};
use r3bl_rs_utils_core::{log_error, CommonError, CommonErrorType, CommonResult};
use std::process::Command;

pub fn try_delete_branch() -> CommonResult<()> {
    giti_ui_templates::multi_select_instruction_header();

    let branches = try_execute_git_command_to_get_branches()?;

    let maybe_selected_branches = ask_user_to_select_from_list(
        branches,
        "Please select branches you want to delete".to_string(),
        SelectionMode::Multiple,
    );

    if let SelectModeResult::Multiple(branches) = maybe_selected_branches {
        let branches_to_delete = branches.join(", ");
        let num_of_branches = branches.len();

        let (confirm_branch_deletion_header, confirm_deletion_options) = {
            let mut confirm_deletion_options: Vec<String> = vec![EXIT.to_string()];
            if num_of_branches == 1 {
                confirm_deletion_options.insert(0, DELETE_BRANCH.to_string());
                (
                    format!("Confirm deleting 1 branch: {}", branches_to_delete),
                    confirm_deletion_options,
                )
            } else {
                confirm_deletion_options.insert(0, DELETE_BRANCHES.to_string());
                (
                    format!(
                        "Confirm deleting {} branches: {}?",
                        num_of_branches, branches_to_delete
                    ),
                    confirm_deletion_options,
                )
            }
        };

        let maybe_selected_delete_or_exit = ask_user_to_select_from_list(
            confirm_deletion_options,
            confirm_branch_deletion_header,
            SelectionMode::Single,
        );

        use user_choice::Selection::{self, *};

        match user_choice::Selection::from(maybe_selected_delete_or_exit) {
            user_choice::Selection::Delete => {
                let mut command =
                    inner::run_git_command_to_delete_branches_on_all_branches(&branches);
                let output = command.output()?;
                if output.status.success() {
                    if num_of_branches == 1 {
                        inner::display_one_branch_deleted_success_message(&branches);
                    } else {
                        inner::display_all_branches_deleted_success_messages(&branches);
                    }
                } else {
                    inner::display_correct_error_message(branches, output);
                }
            }

            user_choice::Selection::Exit => {
                giti_ui_templates::show_exit_message();
            }
        }
    }
    return Ok(());

    mod user_choice {
        use super::*;

        pub enum Selection {
            Delete,
            Exit,
        }

        impl From<SelectModeResult> for Selection {
            fn from(selected: SelectModeResult) -> Selection {
                match selected {
                    SelectModeResult::None => Selection::Exit,
                    SelectModeResult::Single(user_input) => {
                        return if user_input == DELETE_BRANCH.to_string() {
                            Selection::Delete
                        }
                        else { Selection::Exit }
                    }
                    SelectModeResult::Multiple(user_inputs) => {
                        return if user_inputs[0] == DELETE_BRANCHES.to_string() {
                            Selection::Delete
                        }
                        else { Selection::Exit }
                    }
                }
            }
        }
    }

    mod inner {
        use super::*;

        pub fn display_correct_error_message(branches: Vec<String>, output: std::process::Output) {
            if branches.len() == 1 {
                let branch = &branches[0];
                AnsiStyledText {
                    text: &format!(
                        "Failed to delete branch: {}!\n\n{}",
                        branch,
                        String::from_utf8_lossy(&output.stderr)
                    ),
                    style: &[Style::Foreground(FAILED_COLOR)],
                }
                .println();
            } else {
                let branches = branches.join(",\n ╴");
                AnsiStyledText {
                    text: &format!(
                        "Failed to delete branches:\n ╴{}!\n\n{}",
                        branches,
                        String::from_utf8_lossy(&output.stderr)
                    ),
                    style: &[Style::Foreground(FAILED_COLOR)],
                }
                .println();
            }
        }

        pub fn run_git_command_to_delete_branches_on_all_branches(
            branches: &Vec<String>,
        ) -> Command {
            let mut command = Command::new("git");
            command.args(["branch", "-D"]);
            for branch in branches {
                command.arg(branch);
            }
            command
        }

        pub fn display_one_branch_deleted_success_message(branches: &Vec<String>) {
            let branch_name = &branches[0].to_string();
            let deleted_branch = AnsiStyledText {
                text: branch_name,
                style: &[Style::Foreground(SUCCESS_COLOR)],
            };
            let deleted = AnsiStyledText {
                text: "deleted",
                style: &[Style::Foreground(LIGHT_GRAY_COLOR)],
            };
            AnsiStyledText {
                text: &format!("✅ {} {}", deleted_branch, deleted).as_str(),
                style: &[Style::Foreground(SUCCESS_COLOR)],
            }
            .println();
        }

        pub fn display_all_branches_deleted_success_messages(branches: &Vec<String>) {
            for branch in branches {
                let deleted_branch = AnsiStyledText {
                    text: branch,
                    style: &[Style::Foreground(SUCCESS_COLOR)],
                };
                let deleted = AnsiStyledText {
                    text: "deleted",
                    style: &[Style::Foreground(LIGHT_GRAY_COLOR)],
                };
                AnsiStyledText {
                    text: &format!("✅ {} {}", deleted_branch, deleted).as_str(),
                    style: &[Style::Foreground(SUCCESS_COLOR)],
                }
                .println();
            }
        }
    }
}

pub fn try_execute_git_command_to_get_branches() -> CommonResult<Vec<String>> {
    let command: [&str; 3] = ["branch", "--format", "%(refname:short)"];
    match Command::new("git").args(command).output() {
        // Problem executing `git branch --format ...`.
        Err(error) => {
            log_error(format!(
                "Error executing `{}`, error: {:?}",
                command.join(" "),
                error
            ));
            CommonError::new_err_with_only_type(CommonErrorType::CommandExecutionError)
        }
        Ok(output) => {
            let output_string = String::from_utf8_lossy(&output.stdout);
            let mut branches = vec![];
            for line in output_string.lines() {
                branches.push(line.to_string());
            }
            Ok(branches)
        }
    }
}

pub mod giti_ui_templates {
    use super::*;
    use r3bl_rs_utils_core::ch;
    use r3bl_rs_utils_core::ChUnit;

    pub fn multi_select_instruction_header() {
        AnsiStyledText {
            text: &format!(
                "{}{}{}{}",
                "┆ Up or Down:      navigate\n",
                "┆ Space:           select or unselect branches\n",
                "┆ Return:          confirm selection\n",
                "┆ Esc:             exit program\n",
            ),
            style: &[Style::Foreground(LIGHT_GRAY_COLOR)],
        }
        .println();
    }

    pub fn single_select_instruction_header() {
        AnsiStyledText {
            text: &format!(
                "{}{}{}",
                "┆ Up or Down:      navigate\n",
                "┆ Return:          confirm selection\n",
                "┆ Esc:             exit program\n",
            ),
            style: &[Style::Foreground(LIGHT_GRAY_COLOR)],
        }
        .println();
    }

    pub fn ask_user_to_select_from_list(
        options: Vec<String>,
        header: String,
        selection_mode: SelectionMode,
    ) -> SelectModeResult {
        let max_height_row_count = 20;
        let max_width_col_count = get_size().map(|it| it.col_count).unwrap_or(ch!(80)).into();
        let style = StyleSheet::default();
        let user_input = select_from_list(
            header,
            options,
            max_height_row_count,
            max_width_col_count,
            selection_mode,
            style,
        );
        user_input
    }

    pub fn show_exit_message() {
        let text = &{
            format!("You chose to not to delete any branches.\nGoodbye, {}! Thank you for using giti 🐈.\nPlease star r3bl-open-core repo on GitHub!", get_username())
        };
        AnsiStyledText {
            text,
            style: &[Style::Foreground(DUSTY_LIGHT_BLUE_COLOR)],
        }
        .println();

        AnsiStyledText {
            text: "🌟 https://github.com/r3bl-org/r3bl-open-core\n",
            style: &[Style::Bold, Style::Foreground(Color::Rgb(255, 216, 100))],
        }
        .println();
    }

    pub fn get_username() -> String {
        std::env::var("USER").unwrap_or("unknown".to_string())
    }
}
