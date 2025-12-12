// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::{DefaultIoDevices, ItemsOwned, TuiColor, choose, cli_text_inline,
               get_terminal_width, height, new_style,
               readline_async::{HowToChoose, StyleSheet},
               set_mimalloc_in_main, usize, width};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

const JSON_DATA: &str = r#"[
  {
    "question": "What is the capital of France?",
    "options": [
      "Paris",
      "London",
      "Berlin"
    ],
    "correct_answer": "Paris"
  },
  {
    "question": "What is the capital of Estonia?",
    "options": [
      "Oslo",
      "Latvia",
      "Tallinn"
    ],
    "correct_answer": "Tallinn"
  },
  {
    "question": "What is the capital of United States?",
    "options": [
      "Berlin",
      "Washington, D.C.",
      "Ottawa"
    ],
    "correct_answer": "Washington, D.C."
  },
  {
    "question": "What is the capital of India?",
    "options": [
      "New Delhi",
      "Dublin",
      "Dhaka"
    ],
    "correct_answer": "New Delhi"
  }
]"#;

#[derive(Deserialize, Serialize)]
struct QuestionData {
    question: String,
    options: Vec<String>,
    correct_answer: String,
}

#[tokio::main]
pub async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();

    // Parse string into Vec<QuestionData>
    let all_questions_and_answers: Vec<QuestionData> =
        serde_json::from_str(JSON_DATA).unwrap();
    // Get display size.
    let max_width_col_count = usize(*get_terminal_width());
    let max_height_row_count: usize = 5;

    let mut score = 0;
    let correct_answer_color = TuiColor::Rgb((255, 216, 9).into());
    let incorrect_answer_color = TuiColor::Rgb((255, 70, 30).into());
    let line_length = 60;

    display_header(line_length);

    let mut io_devices = DefaultIoDevices::default();

    for question_data in &all_questions_and_answers {
        let question = question_data.question.clone();
        let options = question_data.options.clone();
        let user_input = choose(
            question,
            options,
            Some(height(max_height_row_count)),
            Some(width(max_width_col_count)),
            HowToChoose::Single,
            StyleSheet::default(),
            io_devices.as_mut_tuple(),
        )
        .await?;

        if user_input.is_empty() {
            println!("You did not select anything");
            // Exit the game.
            break;
        }

        check_user_input_and_display_result(
            &user_input,
            question_data,
            correct_answer_color,
            incorrect_answer_color,
            &mut score,
            &all_questions_and_answers,
        );
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
            Answer::Correct => TuiColor::Rgb((5, 236, 0).into()),
            Answer::Incorrect => TuiColor::Rgb((234, 0, 196).into()),
        };

        let text = match self {
            Answer::Correct => "Correct",
            Answer::Incorrect => "Incorrect",
        };

        write!(
            f,
            "{}",
            cli_text_inline(text, new_style!(color_fg: {color}))
        )
    }
}

fn check_answer(guess: &QuestionData, maybe_user_input: Option<&ItemsOwned>) -> Answer {
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
    let color = TuiColor::Rgb((9, 183, 238).into());
    println!();
    println!();
    cli_text_inline(
        "👋 Welcome to the Simple Quiz with choose",
        new_style!(color_fg: {color}),
    )
    .println();

    cli_text_inline(
        "To request_shutdown the game, press 'Esc'",
        new_style!(color_fg: {color}),
    )
    .println();

    cli_text_inline(
        "─".to_string().as_str().repeat(line_length).as_str(),
        new_style!(color_fg: {color}),
    )
    .println();
}

fn display_footer(
    score: i32,
    all_questions_and_answers: &[QuestionData],
    line_length: usize,
) {
    let line = "─".to_string().as_str().repeat(line_length - 2);
    let color = TuiColor::Rgb((9, 183, 238).into());

    cli_text_inline(format!("╭{line}╮").as_str(), new_style!(color_fg: {color}))
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

    cli_text_inline(score_text.join("").as_str(), new_style!(color_fg: {color}))
        .println();

    cli_text_inline(format!("╰{line}╯").as_str(), new_style!(color_fg: {color}))
        .println();
}

fn check_user_input_and_display_result(
    user_input: &ItemsOwned,
    question_data: &QuestionData,
    correct_answer_color: TuiColor,
    incorrect_answer_color: TuiColor,
    score: &mut i32,
    all_questions_and_answers: &[QuestionData],
) {
    let answer = check_answer(question_data, Some(user_input));

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

    let user_input_str = match user_input.first() {
        Some(input) => input,
        None => "No answer",
    };

    println!(
        "{a} {b} {c}",
        a = cli_text_inline(
            format!("{}. {}", question_number, &question_data.question),
            new_style!(color_bg: {background_color}),
        ),
        b = user_input_str,
        c = correct_or_incorrect
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::{ColorSupport, global_color_support};
    use serial_test::serial;

    #[serial]
    #[test]
    fn test_answer_display_correct() {
        global_color_support::set_override(ColorSupport::Truecolor);
        let answer = Answer::Correct;
        // RGB format uses colons per ITU-T Rec. T.416: ESC[38:2:r:g:bm
        let expected_output = "\u{001b}[38:2:5:236:0mCorrect\u{001b}[0m";
        assert_eq!(format!("{}", answer), expected_output);
    }

    #[serial]
    #[test]
    fn test_answer_display_incorrect() {
        global_color_support::set_override(ColorSupport::Truecolor);
        let answer = Answer::Incorrect;
        // RGB format uses colons per ITU-T Rec. T.416: ESC[38:2:r:g:bm
        let expected_output = "\u{001b}[38:2:234:0:196mIncorrect\u{001b}[0m";
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

        let correct_answer = Some(ItemsOwned::from(vec!["Paris".to_string()]));
        let result = check_answer(&guess, correct_answer.as_ref());
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

        let incorrect_answer = Some(ItemsOwned::from(vec!["London".to_string()]));
        let result = check_answer(&guess, incorrect_answer.as_ref());
        assert_eq!(result, Answer::Incorrect);
    }
}
