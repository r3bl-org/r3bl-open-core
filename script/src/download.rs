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

use std::{fs, io::Write as _, path::Path};

use miette::IntoDiagnostic;
use r3bl_core::ok;

use crate::http_client::create_client_with_user_agent;

pub async fn try_download_file_overwrite_existing(
    source_url: &str,
    destination_file: impl AsRef<Path>,
) -> miette::Result<()> {
    let destination = destination_file.as_ref();

    let client = create_client_with_user_agent(None)?;
    let response = client.get(source_url).send().await.into_diagnostic()?;
    let response = response.error_for_status().into_diagnostic()?;
    let response = response.bytes().await.into_diagnostic()?;

    let mut dest_file = fs::File::create(destination).into_diagnostic()?;
    dest_file.write_all(&response).into_diagnostic()?;

    ok!()
}

#[cfg(test)]
mod tests_download {
    use r3bl_core::create_temp_dir;

    use super::*;

    #[tokio::test]
    async fn test_download_file_overwrite_existing() {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_download_file_overwrite_existing");
        fs::create_dir_all(&new_dir).unwrap();

        let source_url = "https://github.com/cloudflare/cfssl/releases/download/v1.6.5/cfssljson_1.6.5_linux_amd64";
        let destination_file = new_dir.join("cfssljson");

        // Download file (no pre-existing file).
        try_download_file_overwrite_existing(source_url, &destination_file)
            .await
            .unwrap();
        assert!(destination_file.exists());

        let meta_data = destination_file.metadata().unwrap();
        let og_file_size = meta_data.len();

        // Download file again (overwrite existing).
        try_download_file_overwrite_existing(source_url, &destination_file)
            .await
            .unwrap();
        assert!(destination_file.exists());

        // Ensure that the file sizes are the same.
        let meta_data = destination_file.metadata().unwrap();
        let new_file_size = meta_data.len();
        assert_eq!(og_file_size, new_file_size);
    }
}
