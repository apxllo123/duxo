use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use walkdir::WalkDir;

use crate::manifest::{ExecutableRecord, FileRecord, GameManifest, ProtectionSignals};

pub fn analyze_game(root: &Path) -> Result<GameManifest> {
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve game path: {}", root.display()))?;
    if !root.is_dir() {
        anyhow::bail!("input is not a directory: {}", root.display());
    }
    let mut files = Vec::new();
    let mut executables = Vec::new();
    for entry in WalkDir::new(&root).follow_links(false) {
        let entry = entry.with_context(|| "failed while walking game directory")?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let relative = path.strip_prefix(&root).unwrap_or(path).to_path_buf();
        let metadata = entry
            .metadata()
            .with_context(|| format!("failed to stat {}", path.display()))?;
        let sha256 = sha256_file(path)?;
        let extension = path
            .extension()
            .and_then(|v| v.to_str())
            .map(str::to_ascii_lowercase);
        files.push(FileRecord {
            path: relative.clone(),
            size: metadata.len(),
            sha256,
            extension: extension.clone(),
        });
        if matches!(extension.as_deref(), Some("exe" | "dll" | "sys")) {
            executables.push(ExecutableRecord {
                path: relative.clone(),
                format: detect_binary_format(path)?,
                architecture: detect_pe_architecture(path)?,
                protection: detect_protection_signals(path, &relative)?,
            });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    executables.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(GameManifest {
        root,
        files,
        executables,
    })
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn detect_binary_format(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut header = [0_u8; 4];
    let read = file.read(&mut header)?;
    Ok(if read >= 2 && &header[..2] == b"MZ" {
        "PE".to_owned()
    } else {
        "unknown".to_owned()
    })
}

fn detect_pe_architecture(path: &Path) -> Result<Option<String>> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut dos_header = [0_u8; 0x40];
    let read = file.read(&mut dos_header)?;
    if read < dos_header.len() || &dos_header[..2] != b"MZ" {
        return Ok(None);
    }
    let pe_offset = u32::from_le_bytes(dos_header[0x3c..0x40].try_into().unwrap()) as u64;
    file.seek(SeekFrom::Start(pe_offset))?;
    let mut pe_header = [0_u8; 6];
    let read = file.read(&mut pe_header)?;
    if read < pe_header.len() || &pe_header[..4] != b"PE\0\0" {
        return Ok(None);
    }
    let machine = u16::from_le_bytes(pe_header[4..6].try_into().unwrap());
    let arch = match machine {
        0x014c => "x86",
        0x8664 => "x86_64",
        0xAA64 => "arm64",
        0x01c4 => "arm32",
        _ => return Ok(Some(format!("machine-0x{machine:04x}"))),
    };
    Ok(Some(arch.to_owned()))
}

fn detect_protection_signals(path: &Path, relative_path: &Path) -> Result<ProtectionSignals> {
    let lower_name = relative_path.to_string_lossy().to_ascii_lowercase();
    let mut protection = ProtectionSignals::default();
    let packer_markers = [
        (b"upx0".as_slice(), "UPX"),
        (b"upx1".as_slice(), "UPX"),
        (b"upx2".as_slice(), "UPX"),
        (b"aspack".as_slice(), "ASPack"),
        (b"mpress".as_slice(), "MPRESS"),
        (b"themida".as_slice(), "Themida"),
        (b"vmprotect".as_slice(), "VMProtect"),
        (b"enigma".as_slice(), "Enigma Protector"),
    ];
    let anti_cheat_markers = [
        (b"easyanticheat".as_slice(), "Easy Anti-Cheat"),
        (b"easyanticheat_eos".as_slice(), "Easy Anti-Cheat EOS"),
        (b"battleye".as_slice(), "BattlEye"),
        (b"bedaisy".as_slice(), "BattlEye"),
        (b"vgk".as_slice(), "Riot Vanguard"),
        (b"vanguard".as_slice(), "Riot Vanguard"),
        (b"gameguard".as_slice(), "nProtect GameGuard"),
        (b"xigncode".as_slice(), "XIGNCODE3"),
    ];
    scan_ascii_markers(path, &packer_markers, &anti_cheat_markers, &mut protection)?;
    for (marker, label) in anti_cheat_markers {
        if contains_ascii_case_insensitive(lower_name.as_bytes(), marker)
            && !protection.anti_cheats.iter().any(|v| v == label)
        {
            protection.anti_cheats.push(label.to_owned());
        }
    }
    protection.packers_or_protectors.sort();
    protection.anti_cheats.sort();
    Ok(protection)
}

fn scan_ascii_markers(
    path: &Path,
    packer_markers: &[(&[u8], &str)],
    anti_cheat_markers: &[(&[u8], &str)],
    protection: &mut ProtectionSignals,
) -> Result<()> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let max_marker_len = packer_markers
        .iter()
        .chain(anti_cheat_markers.iter())
        .map(|(marker, _)| marker.len())
        .max()
        .unwrap_or(1);
    let mut carry = Vec::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let mut chunk = Vec::with_capacity(carry.len() + read);
        chunk.extend_from_slice(&carry);
        chunk.extend_from_slice(&buffer[..read]);
        for (marker, label) in packer_markers {
            if contains_ascii_case_insensitive(&chunk, marker)
                && !protection.packers_or_protectors.iter().any(|v| v == label)
            {
                protection.packers_or_protectors.push((*label).to_owned());
            }
        }
        for (marker, label) in anti_cheat_markers {
            if contains_ascii_case_insensitive(&chunk, marker)
                && !protection.anti_cheats.iter().any(|v| v == label)
            {
                protection.anti_cheats.push((*label).to_owned());
            }
        }
        let keep = max_marker_len.saturating_sub(1).min(chunk.len());
        carry.clear();
        carry.extend_from_slice(&chunk[chunk.len() - keep..]);
    }
    Ok(())
}

fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(&left, &right)| left.eq_ignore_ascii_case(&right))
    })
}

#[cfg(test)]
mod tests {
    use super::{contains_ascii_case_insensitive, detect_protection_signals};
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detects_known_protection_markers_without_modifying_input() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("akron-protection-{unique}.exe"));
        let data = b"This contains UPX0, VMProtect, and EasyAntiCheat markers.";
        fs::write(&path, data).expect("write fixture");
        let before = fs::read(&path).expect("read fixture");
        let signals =
            detect_protection_signals(&path, Path::new("fixture.exe")).expect("detect signals");
        let after = fs::read(&path).expect("read fixture after analysis");
        assert_eq!(before, after);
        assert!(signals.packers_or_protectors.contains(&"UPX".to_owned()));
        assert!(
            signals
                .packers_or_protectors
                .contains(&"VMProtect".to_owned())
        );
        assert!(signals.anti_cheats.contains(&"Easy Anti-Cheat".to_owned()));
        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn detects_markers_split_across_read_boundaries() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("akron-protection-boundary-{unique}.exe"));
        let mut data = vec![b'A'; 1024 * 1024 - 3];
        data.extend_from_slice(b"UPX0");
        fs::write(&path, data).expect("write fixture");
        let signals =
            detect_protection_signals(&path, Path::new("fixture.exe")).expect("detect signals");
        assert!(signals.packers_or_protectors.contains(&"UPX".to_owned()));
        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn ascii_marker_matching_is_case_insensitive() {
        assert!(contains_ascii_case_insensitive(b"vMpRoTeCt", b"vmprotect"));
        assert!(!contains_ascii_case_insensitive(
            b"ordinary data",
            b"vmprotect"
        ));
    }

    #[test]
    fn ignores_unrelated_data() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("akron-protection-clean-{unique}.exe"));
        fs::write(&path, b"ordinary executable data").expect("write fixture");
        let signals =
            detect_protection_signals(&path, Path::new("fixture.exe")).expect("detect signals");
        assert!(signals.packers_or_protectors.is_empty());
        assert!(signals.anti_cheats.is_empty());
        fs::remove_file(path).expect("remove fixture");
    }
}
