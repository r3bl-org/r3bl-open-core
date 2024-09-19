/*
 *   Copyright (c) 2024 R3BL LLC
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

use std::str::FromStr;

/// Used to parse the command line arguments (provided by `clap` crate.
#[derive(Clone, Debug, PartialEq)]
pub enum WriterArg {
    Stdout,
    File,
    None,
}

/// Handle converting parsed command line arguments (via `clap` crate) into a [WriterArg].
///
/// - This is an intermediate representation (IR).
/// - It is ultimately converted into [crate::WriterConfig] by its [TryFrom] trait
///   implementation before it used in the rest of the system.
///
/// This is the intended use case:
///
/// 1. Typically, the `clap` crate parses this into a string.
/// 2. This trait implementation then converts it into a [WriterArg].
impl FromStr for WriterArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stdout" => Ok(WriterArg::Stdout),
            "file" => Ok(WriterArg::File),
            "none" => Ok(WriterArg::None),
            "" => Ok(WriterArg::None),
            _ => Err(format!("{} is not a valid tracing writer", s)),
        }
    }
}

#[cfg(test)]
mod tests_writer_arg {
    use std::str::FromStr as _;

    use crate::tracing_logging::writer_arg::WriterArg;

    #[test]
    fn test_from_str() {
        assert_eq!(WriterArg::from_str("stdout").unwrap(), WriterArg::Stdout);
        assert_eq!(WriterArg::from_str("file").unwrap(), WriterArg::File);
    }

    #[test]
    fn test_invalid_from_str() {
        assert!(WriterArg::from_str("invalid").is_err());
    }
}
