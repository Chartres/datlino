//! OCR dispatch for image-only PDFs (brief §5.1, §13 week 5).
//!
//! Backends:
//! * **macOS** — Apple Vision via a small Swift/ObjC helper would be ideal
//!   (free, CZ/SK-trained). Not wired yet; the macOS build falls through
//!   to Tesseract for now.
//! * **Everywhere else** — Tesseract via the system `tesseract` binary,
//!   with `pdftoppm` (poppler-utils) to rasterise pages.
//!
//! Neither binary is bundled; the user installs via `brew install tesseract
//! poppler` / `apt install tesseract-ocr tesseract-ocr-ces poppler-utils`
//! / Chocolatey. Missing binaries produce a diagnostic, not a panic — the
//! PDF path still ingests whatever text layer was present.
//!
//! Heuristic: an image-only PDF is one where the text layer yields under
//! 120 UTF-8 chars per page on average. Typed PDFs usually deliver 2000+
//! chars per page; anything below the floor is almost certainly scanned.

use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;

/// Language hint for Tesseract. Czech + Slovak + English covers the
/// common high-school case. Each requires the corresponding tessdata
/// file; missing language packs make Tesseract fail with a clear error.
const DEFAULT_LANGUAGES: &str = "ces+slk+eng";

pub fn is_available() -> bool {
    binary_on_path("tesseract") && binary_on_path("pdftoppm")
}

pub struct OcrStatus {
    pub tesseract: bool,
    pub pdftoppm: bool,
}

pub fn status() -> OcrStatus {
    OcrStatus {
        tesseract: binary_on_path("tesseract"),
        pdftoppm: binary_on_path("pdftoppm"),
    }
}

fn binary_on_path(name: &str) -> bool {
    which(name).is_some()
}

fn which(name: &str) -> Option<std::path::PathBuf> {
    // Minimal pure-stdlib `which` — walks $PATH and checks executable bit.
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&candidate) {
                if meta.is_file() && meta.permissions().mode() & 0o111 != 0 {
                    return Some(candidate);
                }
            }
        }
        #[cfg(target_family = "windows")]
        {
            for ext in ["", ".exe", ".cmd", ".bat"] {
                let mut cand = candidate.clone();
                cand.set_extension(ext.trim_start_matches('.'));
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
    }
    None
}

/// Decide whether a PDF is probably image-only based on the text-layer yield.
pub fn looks_image_only(extracted_chars: usize, page_count: usize) -> bool {
    if page_count == 0 {
        return extracted_chars < 120;
    }
    (extracted_chars / page_count.max(1)) < 120
}

/// Run `pdftoppm` → `tesseract` over the PDF and return the concatenated
/// text. Returns an explanatory error if either binary is missing so the
/// caller can surface it to the user instead of silently producing no
/// chunks.
pub fn extract_with_tesseract(pdf_path: &Path, languages: Option<&str>) -> Result<String> {
    if !is_available() {
        return Err(anyhow!(
            "OCR binaries missing — install `tesseract` and `pdftoppm` \
             (poppler-utils). Then try re-ingesting the file."
        ));
    }
    let langs = languages.unwrap_or(DEFAULT_LANGUAGES);
    let tmp = tempfile::TempDir::new()?;
    let stem = tmp.path().join("page");

    // Rasterise every page to PNG at 300 DPI (OCR-friendly).
    let status = Command::new("pdftoppm")
        .args(["-r", "300", "-png"])
        .arg(pdf_path)
        .arg(&stem)
        .status()
        .map_err(|e| anyhow!("pdftoppm spawn: {e}"))?;
    if !status.success() {
        return Err(anyhow!("pdftoppm failed rasterising {}", pdf_path.display()));
    }

    // Gather the generated PNGs in order.
    let mut pngs: Vec<_> = std::fs::read_dir(tmp.path())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("png"))
        .collect();
    pngs.sort();

    let mut out = String::new();
    for page in pngs {
        let result = Command::new("tesseract")
            .arg(&page)
            .arg("-") // stdout
            .args(["-l", langs])
            .output()
            .map_err(|e| anyhow!("tesseract spawn: {e}"))?;
        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(anyhow!("tesseract failed: {stderr}"));
        }
        out.push_str(std::str::from_utf8(&result.stdout).unwrap_or(""));
        out.push_str("\n\n");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_only_heuristic_triggers_on_sparse_text() {
        // A 10-page PDF that yielded 500 chars = 50 chars/page → scanned.
        assert!(looks_image_only(500, 10));
        // A 1-page PDF with 3000 chars — definitely text-layered.
        assert!(!looks_image_only(3000, 1));
    }

    #[test]
    fn status_is_deterministic_and_cheap() {
        let s = status();
        // Test environment may or may not have the binaries; the point is
        // the function returns without error.
        let _ = (s.tesseract, s.pdftoppm);
    }
}
