/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::{fmt::Display, io::Result};

use r3bl_ansi_color::{self, ASTColor, AnsiStyledText};
use r3bl_core::{get_terminal_width, usize};
use r3bl_tuify::{select_from_list, SelectionMode, StyleSheet};
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
    let all_questions_and_answers: Vec<QuestionData> =
        serde_json::from_str(json_data).unwrap();
    // Get display size.
    let max_width_col_count = usize(*get_terminal_width());
    let max_height_row_count: usize = 5;

    let mut score = 0;
    let correct_answer_color = ASTColor::Rgb(255, 216, 9);
    let incorrect_answer_color = ASTColor::Rgb(255, 70, 30);
    let line_length = 60;

    display_header(line_length);

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
            Answer::Correct => ASTColor::Rgb(5, 236, 0),
            Answer::Incorrect => ASTColor::Rgb(234, 0, 196),
        };

        let text = match self {
            Answer::Correct => "Correct",
            Answer::Incorrect => "Incorrect",
        };

        write!(
            f,
            "{}",
            AnsiStyledText {
                text,
                style: smallvec::smallvec![r3bl_ansi_color::ASTStyle::Foreground(color)],
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
    let color = ASTColor::Rgb(9, 183, 238);
    println!();
    println!();
    AnsiStyledText {
        text: "👋 Welcome to the Simple Quiz with Tuify",
        style: smallvec::smallvec![r3bl_ansi_color::ASTStyle::Foreground(color)],
    }
    .println();

    AnsiStyledText {
        text: "To exit the game, press 'Esc'",
        style: smallvec::smallvec![r3bl_ansi_color::ASTStyle::Foreground(color)],
    }
    .println();

    AnsiStyledText {
        text: "─".to_string().as_str().repeat(line_length).as_str(),
        style: smallvec::smallvec![r3bl_ansi_color::ASTStyle::Foreground(color)],
    }
    .println();
}

fn display_footer(
    score: i32,
    all_questions_and_answers: &[QuestionData],
    line_length: usize,
) {
    let line = "─".to_string().as_str().repeat(line_length - 2);
    let color = ASTColor::Rgb(9, 183, 238);

    AnsiStyledText {
        text: format!("╭{}╮", line).as_str(),
        style: smallvec::smallvec![r3bl_ansi_color::ASTStyle::Foreground(color)],
    }
    .println();

    let vertical_line = "│".to_string();
    let mut score_text = Vec::<String>::new();
    score_text.push(vertical_line.clone());
    score_text.push(format!(
        " End of the game: Your score is {}/{}",
        score,
        all_questions_and_answers.len()
    ));

    let text_length = score_text.join("").len();
    let spaces_to_add = line_length - text_length + 1;
    score_text.push(" ".to_string().repeat(spaces_to_add));
    score_text.push(vertical_line.clone());

    AnsiStyledText {
        text: score_text.join("").as_str(),
        style: smallvec::smallvec![r3bl_ansi_color::ASTStyle::Foreground(color)],
    }
    .println();

    AnsiStyledText {
        text: format!("╰{}╯", line).as_str(),
        style: smallvec::smallvec![r3bl_ansi_color::ASTStyle::Foreground(color)],
    }
    .println();
}

fn check_user_input_and_display_result(
    input: &[String],
    question_data: &QuestionData,
    user_input: &Option<Vec<String>>,
    correct_answer_color: ASTColor,
    incorrect_answer_color: ASTColor,
    score: &mut i32,
    all_questions_and_answers: &[QuestionData],
) {
    let answer = check_answer(question_data, user_input);

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
            style: smallvec::smallvec![r3bl_ansi_color::ASTStyle::Foreground(
                background_color
            )],
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
