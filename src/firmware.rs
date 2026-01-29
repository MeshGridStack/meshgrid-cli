use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const GITHUB_REPO: &str = "MeshGridStack/meshgrid-firmware";
const GITHUB_API_BASE: &str = "https://api.github.com";

/// GitHub release information
#[derive(Debug, Deserialize, Serialize)]
pub struct Release {
    pub tag_name: String,
    pub name: String,
    pub assets: Vec<Asset>,
}

/// GitHub release asset
#[derive(Debug, Deserialize, Serialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

/// Firmware manager for downloading and verifying firmware from GitHub
pub struct FirmwareManager {
    client: Client,
    cache_dir: PathBuf,
}

impl FirmwareManager {
    /// Create a new firmware manager
    pub fn new() -> Result<Self> {
        let cache_dir = Self::get_cache_dir()?;
        fs::create_dir_all(&cache_dir).context("Failed to create firmware cache directory")?;

        let client = Client::builder()
            .user_agent("meshgrid-cli")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, cache_dir })
    }

    /// Get the cache directory path
    fn get_cache_dir() -> Result<PathBuf> {
        let cache_base =
            dirs::cache_dir().ok_or_else(|| anyhow!("Could not determine cache directory"))?;
        Ok(cache_base.join("meshgrid-cli").join("firmware"))
    }

    /// Get firmware for a specific environment and version
    /// Returns the path to the firmware binary
    pub async fn get_firmware(
        &self,
        env_name: &str,
        version: &str,
        force_download: bool,
        offline: bool,
    ) -> Result<PathBuf> {
        let version = if version == "latest" {
            if offline {
                return Err(anyhow!(
                    "Cannot use 'latest' version in offline mode\n\
                     Please specify a specific version or remove --offline"
                ));
            }
            self.get_latest_version().await?
        } else {
            version.to_string()
        };

        let firmware_filename = format!("meshgrid-{}-{}.bin", env_name, version);
        let version_dir = self.cache_dir.join(&version);
        let firmware_path = version_dir.join(&firmware_filename);

        // Check cache first
        if firmware_path.exists() && !force_download {
            println!("✓ Using cached firmware: {}", firmware_filename);
            return Ok(firmware_path);
        }

        // In offline mode, only use cache
        if offline {
            return Err(anyhow!(
                "✗ Firmware not found in cache (offline mode)\n\
                 Cached versions: {}\n\
                 Try: meshgrid-cli flash {} --version {} --offline\n\
                 Or remove --offline to download",
                self.list_cached_versions()?.join(", "),
                env_name,
                version
            ));
        }

        // Download and verify
        println!("Downloading firmware version {}...", version);
        self.download_and_verify(&version, env_name, &version_dir, &firmware_filename)
            .await?;

        Ok(firmware_path)
    }

    /// Get the latest release version from GitHub
    async fn get_latest_version(&self) -> Result<String> {
        let release = self.fetch_release("latest").await?;
        Ok(release.tag_name)
    }

    /// Fetch release information from GitHub API
    pub async fn fetch_release(&self, version: &str) -> Result<Release> {
        let url = if version == "latest" {
            format!("{}/repos/{}/releases/latest", GITHUB_API_BASE, GITHUB_REPO)
        } else {
            format!(
                "{}/repos/{}/releases/tags/{}",
                GITHUB_API_BASE, GITHUB_REPO, version
            )
        };

        let mut request = self.client.get(&url);

        // Use GitHub token if available for higher rate limits
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .context("Failed to fetch release info")?;

        if response.status().as_u16() == 404 {
            return Err(anyhow!(
                "Release version '{}' not found\n\
                 Check available versions at: https://github.com/{}/releases",
                version,
                GITHUB_REPO
            ));
        }

        if response.status().as_u16() == 403 {
            return Err(anyhow!(
                "✗ GitHub API rate limit exceeded (60 requests/hour)\n\
                 Set GITHUB_TOKEN for higher limits:\n\
                 export GITHUB_TOKEN=your_token_here\n\n\
                 Or use local firmware:\n\
                 meshgrid-cli flash --local ../meshgrid-firmware"
            ));
        }

        response
            .error_for_status()?
            .json()
            .await
            .context("Failed to parse release info")
    }

    /// Download firmware and checksum, then verify
    async fn download_and_verify(
        &self,
        version: &str,
        env_name: &str,
        version_dir: &Path,
        firmware_filename: &str,
    ) -> Result<()> {
        // Create version directory
        fs::create_dir_all(version_dir).context("Failed to create version cache directory")?;

        let firmware_path = version_dir.join(firmware_filename);
        let checksum_filename = format!("{}.sha256", firmware_filename);
        let checksum_path = version_dir.join(&checksum_filename);

        // Fetch release info to get download URLs
        let release = self.fetch_release(version).await?;

        // Find firmware and checksum assets
        let firmware_asset = release
            .assets
            .iter()
            .find(|a| a.name == firmware_filename)
            .ok_or_else(|| {
                anyhow!(
                    "Firmware binary '{}' not found in release {}\n\
                     Available assets: {}\n\
                     The environment '{}' may not be supported in this release.",
                    firmware_filename,
                    version,
                    release
                        .assets
                        .iter()
                        .map(|a| &a.name)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                    env_name
                )
            })?;

        let checksum_asset = release
            .assets
            .iter()
            .find(|a| a.name == checksum_filename)
            .ok_or_else(|| {
                anyhow!(
                    "Checksum file '{}' not found in release {}",
                    checksum_filename,
                    version
                )
            })?;

        // Download firmware binary with progress bar
        println!("Downloading {}...", firmware_filename);
        self.download_file(&firmware_asset.browser_download_url, &firmware_path)
            .await?;

        // Download checksum file
        println!("Downloading {}...", checksum_filename);
        self.download_file(&checksum_asset.browser_download_url, &checksum_path)
            .await?;

        // Verify checksum
        println!("Verifying SHA256 checksum...");
        self.verify_checksum(&firmware_path, &checksum_path).await?;

        println!("✓ Firmware downloaded and verified successfully");

        Ok(())
    }

    /// Download a file from URL with progress bar
    async fn download_file(&self, url: &str, dest_path: &Path) -> Result<()> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to start download")?
            .error_for_status()
            .context("Download request failed")?;

        let total_size = response.content_length().unwrap_or(0);

        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        let mut file = fs::File::create(dest_path).context("Failed to create destination file")?;
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read download chunk")?;
            file.write_all(&chunk).context("Failed to write to file")?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message("Downloaded");

        Ok(())
    }

    /// Verify SHA256 checksum of firmware
    async fn verify_checksum(&self, firmware_path: &Path, checksum_path: &Path) -> Result<()> {
        // Read expected checksum from file
        let checksum_content =
            fs::read_to_string(checksum_path).context("Failed to read checksum file")?;
        let expected_hash = checksum_content
            .split_whitespace()
            .next()
            .ok_or_else(|| anyhow!("Invalid checksum file format"))?
            .to_lowercase();

        // Compute actual checksum
        let firmware_data = fs::read(firmware_path).context("Failed to read firmware file")?;
        let mut hasher = Sha256::new();
        hasher.update(&firmware_data);
        let actual_hash = format!("{:x}", hasher.finalize());

        // Compare checksums
        if actual_hash != expected_hash {
            // Clean up invalid files
            let _ = fs::remove_file(firmware_path);
            let _ = fs::remove_file(checksum_path);

            return Err(anyhow!(
                "✗ Firmware verification failed: SHA256 checksum mismatch\n\
                 Expected: {}\n\
                 Actual:   {}\n\
                 Downloaded file may be corrupted.\n\
                 Try: meshgrid-cli flash --force-download",
                expected_hash,
                actual_hash
            ));
        }

        Ok(())
    }

    /// List all cached firmware versions
    pub fn list_cached_versions(&self) -> Result<Vec<String>> {
        let mut versions = Vec::new();

        if !self.cache_dir.exists() {
            return Ok(versions);
        }

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(version) = entry.file_name().to_str() {
                    versions.push(version.to_string());
                }
            }
        }

        versions.sort();
        Ok(versions)
    }
}
