use std::{fmt::Display, io::Result, option::Option, string::String};

use r3bl_ansi_color::*;
use r3bl_tuify::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct QuestionData {
    question: String,
    options: Vec<String>,
    correct_answer: String,
}

pub fn main() -> Result<()> {
    // Load string from file single_select_quiz_data.json
    let json_data = include_str!("data/quiz-game-single-select.json");

    // Parse string into Vec<QuestionData>
    let all_questions_and_answers: Vec<QuestionData> = serde_json::from_str(json_data).unwrap();
    // Get display size.
    let max_width_col_count: usize = get_terminal_width();
    let max_height_row_count: usize = 5;

    let mut score = 0;
    let correct_answer_color = Color::Rgb(255, 216, 9);
    let incorrect_answer_color = Color::Rgb(255, 70, 30);
    let line_length = 60;

    display_header(line_length.clone());

    for question_data in &all_questions_and_answers {
        let question = question_data.question.clone();
        let options = question_data.options.clone();

        let user_input = select_from_list(
            question,
            options,
            max_height_row_count,
            max_width_col_count,
            SelectionMode::Single,
            StyleSheet::default(),
        );

        match &user_input {
            Some(input) => {
                check_user_input_and_display_result(
                    input,
                    question_data,
                    &user_input,
                    correct_answer_color,
                    incorrect_answer_color,
                    &mut score,
                    &all_questions_and_answers,
                );
            }
            None => {
                println!("You did not select anything");
                // Exit the game.
                break;
            }
        };
    }

    display_footer(score, &all_questions_and_answers, line_length);

    Ok(())
}

#[derive(Debug, PartialEq)]
enum Answer {
    Correct,
    Incorrect,
}

// Implement the Display trait for the Answer enum.
impl Display for Answer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let color = match self {
            Answer::Correct => Color::Rgb(5, 236, 0),
            Answer::Incorrect => Color::Rgb(234, 0, 196),
        };

        let text = match self {
            Answer::Correct => "Correct",
            Answer::Incorrect => "Incorrect",
        };

        write!(
            f,
            "{}",
            AnsiStyledText {
                text: &text.to_string(),
                style: &[r3bl_ansi_color::Style::Foreground(color)],
            }
        )
    }
}

fn check_answer(guess: &QuestionData, maybe_user_input: &Option<Vec<String>>) -> Answer {
    // If the maybe_user_input has 1 item then proceed. Otherwise return incorrect.
    match maybe_user_input {
        Some(user_input) => {
            let maybe_user_answer = user_input.first();

            match maybe_user_answer {
                Some(user_answer) => {
                    if *user_answer == guess.correct_answer {
                        Answer::Correct
                    } else {
                        Answer::Incorrect
                    }
                }
                None => Answer::Incorrect,
            }
        }
        None => Answer::Incorrect,
    }
}

fn display_header(line_length: usize) {
    let color = Color::Rgb(9, 183, 238);
    println!("");
    println!("");
    AnsiStyledText {
        text: "👋 Welcome to the Simple Quiz with Tuify",
        style: &[r3bl_ansi_color::Style::Foreground(color)],
    }
    .println();

    AnsiStyledText {
        text: "To exit the game, press 'Esc'",
        style: &[r3bl_ansi_color::Style::Foreground(color)],
    }
    .println();

    AnsiStyledText {
        text: "─".to_string().as_str().repeat(line_length).as_str(),
        style: &[r3bl_ansi_color::Style::Foreground(color)],
    }
    .println();
}

fn display_footer(score: i32, all_questions_and_answers: &Vec<QuestionData>, line_length: usize) {
    let line = "─".to_string().as_str().repeat(line_length - 2);
    let color = Color::Rgb(9, 183, 238);

    AnsiStyledText {
        text: format!("╭{}╮", line).as_str(),
        style: &[r3bl_ansi_color::Style::Foreground(color)],
    }
    .println();

    let vertical_line = "│".to_string();
    let mut score_text = Vec::<String>::new();
    score_text.push(vertical_line.clone());
    score_text.push(format!(
        " End of the game: Your score is {}/{}",
        score.to_string(),
        all_questions_and_answers.len()
    ));

    let text_length = score_text.join("").len();
    let spaces_to_add = line_length - text_length + 1;
    score_text.push(" ".to_string().repeat(spaces_to_add));
    score_text.push(vertical_line.clone());

    AnsiStyledText {
        text: score_text.join("").as_str(),
        style: &[r3bl_ansi_color::Style::Foreground(color)],
    }
    .println();

    AnsiStyledText {
        text: format!("╰{}╯", line).as_str(),
        style: &[r3bl_ansi_color::Style::Foreground(color)],
    }
    .println();
}

fn check_user_input_and_display_result(
    input: &Vec<String>,
    question_data: &QuestionData,
    user_input: &Option<Vec<String>>,
    correct_answer_color: Color,
    incorrect_answer_color: Color,
    score: &mut i32,
    all_questions_and_answers: &Vec<QuestionData>,
) {
    let answer = check_answer(&question_data, &user_input);

    let background_color = match answer {
        Answer::Correct => correct_answer_color,
        Answer::Incorrect => incorrect_answer_color,
    };

    let correct_or_incorrect = match answer {
        Answer::Correct => "| 🎉 Correct",
        Answer::Incorrect => "| 👎 Incorrect",
    };

    if let Some(Answer::Correct) = Some(answer) {
        *score += 1;
    }

    let question_number = all_questions_and_answers
        .iter()
        .position(|it| it.question == question_data.question)
        .unwrap()
        + 1;

    println!(
        "{} {} {}",
        AnsiStyledText {
            text: format!("{}. {}", question_number, &question_data.question).as_str(),
            style: &[r3bl_ansi_color::Style::Foreground(background_color)],
        },
        input[0],
        correct_or_incorrect
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_answer_display_correct() {
        let answer = Answer::Correct;
        let expected_output = "\u{001b}[38;2;5;236;0mCorrect\u{001b}[0m";
        assert_eq!(format!("{}", answer), expected_output);
    }

    #[test]
    fn test_answer_display_incorrect() {
        let answer = Answer::Incorrect;
        let expected_output = "\u{001b}[38;2;234;0;196mIncorrect\u{001b}[0m";
        assert_eq!(format!("{}", answer), expected_output);
    }

    #[test]
    fn test_check_answer_correct() {
        let guess = QuestionData {
            question: "What is the capital of France?".to_string(),
            options: vec![
                "London".to_string(),
                "Paris".to_string(),
                "Berlin".to_string(),
            ],
            correct_answer: "Paris".to_string(),
        };

        let correct_answer = Some(vec!["Paris".to_string()]);
        let result = check_answer(&guess, &correct_answer);
        assert_eq!(result, Answer::Correct);
    }

    #[test]
    fn test_check_answer_incorrect() {
        let guess = QuestionData {
            question: "What is the capital of France?".to_string(),
            options: vec![
                "London".to_string(),
                "Paris".to_string(),
                "Berlin".to_string(),
            ],
            correct_answer: "Paris".to_string(),
        };

        let incorrect_answer = Some(vec!["London".to_string()]);
        let result = check_answer(&guess, &incorrect_answer);
        assert_eq!(result, Answer::Incorrect);
    }
}
