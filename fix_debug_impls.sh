#!/bin/bash

#
#   Copyright (c) 2025 R3BL LLC
#   All rights reserved.
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.
#

# This script adds #[derive(Debug)] to all types that are missing it
# It uses the output of cargo clippy to find the affected files

# Run clippy to get the warnings
cargo clippy -- -W missing_debug_implementations 2>&1 > clippy_output.txt

# Extract the file paths and line numbers from the clippy output
grep -n "type does not implement \`std::fmt::Debug\`" clippy_output.txt | while read -r line; do
    line_num=$(echo "$line" | cut -d':' -f1)
    next_line=$((line_num + 1))
    file_info=$(sed -n "${next_line}p" clippy_output.txt)
    
    # Extract file path and line number
    file_path=$(echo "$file_info" | grep -o "[^ ]*:[0-9]*:[0-9]*" | cut -d':' -f1)
    line_num=$(echo "$file_info" | grep -o "[^ ]*:[0-9]*:[0-9]*" | cut -d':' -f2)
    
    if [ -n "$file_path" ] && [ -n "$line_num" ]; then
        echo "Processing $file_path at line $line_num"
        
        # Check if it's a struct or enum
        type_line=$(sed -n "${line_num}p" "$file_path")
        
        if [[ "$type_line" == *"pub struct"* ]] || [[ "$type_line" == *"pub enum"* ]]; then
            # Add #[derive(Debug)] before the type definition
            sed -i "${line_num}i#[derive(Debug)]" "$file_path"
            echo "Added #[derive(Debug)] to $file_path at line $line_num"
        fi
    fi
done

echo "Done! Please review the changes and run cargo clippy again to verify."