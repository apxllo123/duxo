use anyhow::{Context, Result};
use goblin::pe::PE;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeBinaryAnalysis {
    pub is_64: bool,
    pub is_library: bool,
    pub entry_point_rva: u32,
    pub image_base: u64,
    pub libraries: Vec<String>,
    pub imports: Vec<PeImport>,
    pub sections: Vec<PeSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeImport {
    pub library: String,
    pub name: Option<String>,
    pub ordinal: Option<u16>,
    pub rva: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeSection {
    pub name: String,
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub raw_size: u32,
    pub raw_offset: u32,
    pub characteristics: u32,
}

pub fn analyze_pe(path: &Path) -> Result<PeBinaryAnalysis> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let pe = PE::parse(&bytes).with_context(|| format!("failed to parse PE {}", path.display()))?;

    let mut libraries = pe
        .libraries
        .iter()
        .map(|library| library.to_ascii_lowercase())
        .collect::<Vec<_>>();
    libraries.sort();
    libraries.dedup();

    let mut imports = pe
        .imports
        .iter()
        .map(|import| PeImport {
            library: import.dll.to_ascii_lowercase(),
            name: if import.name.is_empty() { None } else { Some(import.name.to_string()) },
            ordinal: (import.ordinal != 0).then_some(import.ordinal),
            rva: import.rva as u64,
        })
        .collect::<Vec<_>>();
    imports.sort_by(|a, b| {
        a.library
            .cmp(&b.library)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.ordinal.cmp(&b.ordinal))
            .then_with(|| a.rva.cmp(&b.rva))
    });

    let mut sections = pe
        .sections
        .iter()
        .map(|section| {
            let name = section.real_name.clone().unwrap_or_else(|| {
                String::from_utf8_lossy(&section.name)
                    .trim_end_matches('\0')
                    .to_owned()
            });
            PeSection {
                name,
                virtual_size: section.virtual_size,
                virtual_address: section.virtual_address,
                raw_size: section.size_of_raw_data,
                raw_offset: section.pointer_to_raw_data,
                characteristics: section.characteristics,
            }
        })
        .collect::<Vec<_>>();
    sections.sort_by_key(|section| section.virtual_address);

    Ok(PeBinaryAnalysis {
        is_64: pe.is_64,
        is_library: pe.is_lib,
        entry_point_rva: pe.entry,
        image_base: pe.image_base,
        libraries,
        imports,
        sections,
    })
}

#[cfg(test)]
mod tests {
    use super::analyze_pe;
    use std::path::Path;

    #[test]
    fn rejects_non_pe_input() {
        let path = std::env::temp_dir().join("duxo-not-a-pe.bin");
        std::fs::write(&path, b"not a portable executable").expect("write fixture");
        let result = analyze_pe(Path::new(&path));
        assert!(result.is_err());
        std::fs::remove_file(path).expect("remove fixture");
    }
}
