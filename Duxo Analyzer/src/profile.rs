use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::manifest::{GameManifest, ProtectionSignals};
use crate::pe::{PeImport, analyze_pe};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameProfile {
    pub executables: Vec<ExecutableProfile>,
    pub pe_binaries: Vec<PeBinaryProfile>,
    pub dependencies: Vec<BinaryDependency>,
    pub graphics: GraphicsRequirements,
    pub windows_apis: Vec<WindowsApiRequirement>,
    pub runtimes: Vec<RuntimeRequirement>,
    pub unresolved_imports: Vec<BinaryDependency>,
    pub protections: ProtectionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutableProfile {
    pub path: String,
    pub architecture: Option<String>,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeBinaryProfile {
    pub path: String,
    pub architecture: String,
    pub kind: String,
    pub import_count: usize,
    pub libraries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BinaryDependency {
    pub importer: String,
    pub library: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphicsRequirements {
    pub direct3d9: bool,
    pub direct3d10: bool,
    pub direct3d11: bool,
    pub direct3d12: bool,
    pub dxgi: bool,
    pub vulkan: bool,
    pub opengl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowsApiRequirement {
    pub family: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeRequirement {
    pub name: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtectionSummary {
    pub packers_or_protectors: Vec<String>,
    pub anti_cheats: Vec<String>,
}

pub fn profile_game(manifest: &GameManifest) -> Result<GameProfile> {
    let mut profile = GameProfile {
        executables: manifest
            .executables
            .iter()
            .map(|exe| ExecutableProfile {
                path: exe.path.to_string_lossy().into_owned(),
                architecture: exe.architecture.clone(),
                format: exe.format.clone(),
            })
            .collect(),
        pe_binaries: Vec::new(),
        dependencies: Vec::new(),
        graphics: GraphicsRequirements::default(),
        windows_apis: Vec::new(),
        runtimes: Vec::new(),
        unresolved_imports: Vec::new(),
        protections: ProtectionSummary::default(),
    };

    let mut api_evidence = BTreeMap::<String, BTreeSet<String>>::new();
    let mut runtime_evidence = BTreeMap::<String, BTreeSet<String>>::new();
    let bundled_files = manifest_bundled_file_names(manifest);
    let mut targets = BTreeSet::<PathBuf>::new();

    for exe in &manifest.executables {
        targets.insert(exe.path.clone());
        merge_protections(&mut profile.protections, &exe.protection);
    }

    // DLLs/SYS files are important because the main EXE often imports the actual
    // graphics/runtime surface through a helper binary rather than directly.
    for file in &manifest.files {
        let extension = file.extension.as_deref().unwrap_or_default();
        if extension.eq_ignore_ascii_case("dll") || extension.eq_ignore_ascii_case("sys") {
            targets.insert(file.path.clone());
        }
    }

    for relative_path in targets {
        let absolute = manifest.root.join(&relative_path);
        let analysis = analyze_pe(&absolute)
            .with_context(|| format!("failed to profile PE {}", absolute.display()))?;

        let architecture = if analysis.is_64 { "x86_64" } else { "x86" }.to_owned();
        let kind = if analysis.is_library { "DLL" } else { "EXE" }.to_owned();
        let libraries = analysis.libraries.clone();
        let importer = relative_path.to_string_lossy().into_owned();

        profile.pe_binaries.push(PeBinaryProfile {
            path: importer.clone(),
            architecture,
            kind,
            import_count: analysis.imports.len(),
            libraries,
        });

        for import in &analysis.imports {
            apply_import_requirements(
                &relative_path,
                import,
                &mut profile.graphics,
                &mut api_evidence,
                &mut runtime_evidence,
            );

            let library = normalize_library(&import.library);
            let dependency = BinaryDependency {
                importer: importer.clone(),
                library: library.clone(),
            };
            profile.dependencies.push(dependency.clone());

            if !is_platform_provided(&library) && !bundled_files.contains(&library) {
                profile.unresolved_imports.push(dependency);
            }
        }
    }

    profile.pe_binaries.sort_by(|a, b| a.path.cmp(&b.path));
    sort_dependencies(&mut profile.dependencies);
    sort_dependencies(&mut profile.unresolved_imports);

    profile.windows_apis = api_evidence
        .into_iter()
        .map(|(family, evidence)| WindowsApiRequirement {
            family,
            evidence: evidence.into_iter().collect(),
        })
        .collect();

    profile.runtimes = runtime_evidence
        .into_iter()
        .map(|(name, evidence)| RuntimeRequirement {
            name,
            evidence: evidence.into_iter().collect(),
        })
        .collect();

    Ok(profile)
}

fn sort_dependencies(dependencies: &mut Vec<BinaryDependency>) {
    dependencies.sort_by(|a, b| {
        a.importer
            .cmp(&b.importer)
            .then_with(|| a.library.cmp(&b.library))
    });
    dependencies.dedup();
}

fn manifest_bundled_file_names(manifest: &GameManifest) -> BTreeSet<String> {
    manifest
        .files
        .iter()
        .filter_map(|file| {
            file.path
                .file_name()
                .map(|name| name.to_string_lossy().to_ascii_lowercase())
        })
        .collect()
}

fn merge_protections(summary: &mut ProtectionSummary, signals: &ProtectionSignals) {
    for value in &signals.packers_or_protectors {
        if !summary.packers_or_protectors.contains(value) {
            summary.packers_or_protectors.push(value.clone());
        }
    }
    for value in &signals.anti_cheats {
        if !summary.anti_cheats.contains(value) {
            summary.anti_cheats.push(value.clone());
        }
    }
    summary.packers_or_protectors.sort();
    summary.anti_cheats.sort();
}

fn apply_import_requirements(
    binary: &Path,
    import: &PeImport,
    graphics: &mut GraphicsRequirements,
    api_evidence: &mut BTreeMap<String, BTreeSet<String>>,
    runtime_evidence: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let library = normalize_library(&import.library);
    let symbol = import.name.as_deref().unwrap_or("ordinal");
    let evidence = format!("{} -> {}!{}", binary.display(), library, symbol);

    match library.as_str() {
        "d3d9.dll" | "d3d9_43.dll" => graphics.direct3d9 = true,
        "d3d10.dll" | "d3d10_1.dll" => graphics.direct3d10 = true,
        "d3d11.dll" => graphics.direct3d11 = true,
        "d3d12.dll" => graphics.direct3d12 = true,
        "dxgi.dll" => graphics.dxgi = true,
        "vulkan-1.dll" => graphics.vulkan = true,
        "opengl32.dll" => graphics.opengl = true,
        _ => {}
    }

    let api_family = match library.as_str() {
        "kernel32.dll" | "kernelbase.dll" => Some("process-threading/filesystem"),
        "user32.dll" => Some("windowing/input"),
        "gdi32.dll" => Some("windowing/2d-graphics"),
        "advapi32.dll" => Some("registry/security/services"),
        "shell32.dll" => Some("shell/integration"),
        "ole32.dll" => Some("com"),
        "ws2_32.dll" => Some("networking"),
        "winhttp.dll" => Some("http-networking"),
        "winmm.dll" => Some("legacy-audio/timing"),
        "xaudio2_9.dll" | "xaudio2_8.dll" | "xaudio2_7.dll" => Some("audio"),
        "xinput1_4.dll" | "xinput1_3.dll" => Some("controller-input"),
        "hid.dll" => Some("raw-input"),
        _ => None,
    };
    if let Some(family) = api_family {
        api_evidence
            .entry(family.to_owned())
            .or_default()
            .insert(evidence.clone());
    }

    let runtime = match library.as_str() {
        "vcruntime140.dll" | "vcruntime140_1.dll" | "msvcp140.dll" => {
            Some("Microsoft Visual C++ 2015-2022")
        }
        "ucrtbase.dll" => Some("Universal C Runtime"),
        "msvcp120.dll" | "msvcr120.dll" => Some("Microsoft Visual C++ 2013"),
        "msvcp110.dll" | "msvcr110.dll" => Some("Microsoft Visual C++ 2012"),
        "mscoree.dll" => Some(".NET Framework CLR"),
        _ => None,
    };
    if let Some(name) = runtime {
        runtime_evidence
            .entry(name.to_owned())
            .or_default()
            .insert(evidence);
    }
}

fn normalize_library(library: &str) -> String {
    library.to_ascii_lowercase()
}

fn is_platform_provided(library: &str) -> bool {
    matches!(
        library,
        "kernel32.dll"
            | "kernelbase.dll"
            | "ntdll.dll"
            | "user32.dll"
            | "gdi32.dll"
            | "advapi32.dll"
            | "shell32.dll"
            | "ole32.dll"
            | "oleaut32.dll"
            | "combase.dll"
            | "ws2_32.dll"
            | "winhttp.dll"
            | "winmm.dll"
            | "hid.dll"
            | "d3d9.dll"
            | "d3d9_43.dll"
            | "d3d10.dll"
            | "d3d10_1.dll"
            | "d3d11.dll"
            | "d3d12.dll"
            | "dxgi.dll"
            | "opengl32.dll"
            | "vulkan-1.dll"
            | "xaudio2_7.dll"
            | "xaudio2_8.dll"
            | "xaudio2_9.dll"
            | "xinput1_3.dll"
            | "xinput1_4.dll"
            | "mscoree.dll"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        BinaryDependency, GameProfile, GraphicsRequirements, profile_game, sort_dependencies,
    };
    use crate::manifest::{ExecutableRecord, FileRecord, GameManifest, ProtectionSignals};
    use std::path::PathBuf;

    #[test]
    fn dependency_graph_is_deterministic_and_deduplicated() {
        let mut dependencies = vec![
            BinaryDependency {
                importer: "z.dll".to_owned(),
                library: "kernel32.dll".to_owned(),
            },
            BinaryDependency {
                importer: "a.exe".to_owned(),
                library: "renderer.dll".to_owned(),
            },
            BinaryDependency {
                importer: "z.dll".to_owned(),
                library: "kernel32.dll".to_owned(),
            },
        ];

        sort_dependencies(&mut dependencies);

        assert_eq!(dependencies.len(), 2);
        assert_eq!(dependencies[0].importer, "a.exe");
        assert_eq!(dependencies[1].importer, "z.dll");
    }

    #[test]
    fn rejects_raw_strings_as_graphics_evidence() {
        let root = std::env::temp_dir().join("duxo-profile-raw-marker");
        std::fs::create_dir_all(&root).expect("create fixture dir");
        let exe = root.join("game.exe");
        std::fs::write(&exe, b"d3d11.dll dxgi.dll vcruntime140.dll").expect("write fixture");

        let manifest = GameManifest {
            root: root.clone(),
            files: Vec::new(),
            executables: vec![ExecutableRecord {
                path: PathBuf::from("game.exe"),
                format: "PE".to_owned(),
                architecture: Some("x86_64".to_owned()),
                protection: ProtectionSignals::default(),
            }],
        };

        assert!(profile_game(&manifest).is_err());
        assert_eq!(
            GraphicsRequirements::default(),
            GameProfile {
                executables: Vec::new(),
                pe_binaries: Vec::new(),
                dependencies: Vec::new(),
                graphics: GraphicsRequirements::default(),
                windows_apis: Vec::new(),
                runtimes: Vec::new(),
                unresolved_imports: Vec::new(),
                protections: Default::default(),
            }
            .graphics
        );

        std::fs::remove_file(exe).expect("remove fixture");
        std::fs::remove_dir(root).expect("remove fixture dir");
    }

    #[test]
    fn malformed_pe_is_not_treated_as_a_valid_profile() {
        let root = std::env::temp_dir().join("duxo-profile-invalid-pe");
        std::fs::create_dir_all(&root).expect("create fixture dir");
        let exe = root.join("game.exe");
        std::fs::write(&exe, b"not a portable executable").expect("write fixture");

        let manifest = GameManifest {
            root: root.clone(),
            files: Vec::new(),
            executables: vec![ExecutableRecord {
                path: PathBuf::from("game.exe"),
                format: "PE".to_owned(),
                architecture: Some("x86_64".to_owned()),
                protection: ProtectionSignals::default(),
            }],
        };

        assert!(profile_game(&manifest).is_err());
        std::fs::remove_file(exe).expect("remove fixture");
        std::fs::remove_dir(root).expect("remove fixture dir");
    }

    #[test]
    fn manifest_dlls_are_included_in_profile_targets() {
        let root = std::env::temp_dir().join("duxo-profile-dll-target");
        std::fs::create_dir_all(&root).expect("create fixture dir");
        let exe = root.join("game.exe");
        let dll = root.join("renderer.dll");
        std::fs::write(&exe, b"not a PE").expect("write exe fixture");
        std::fs::write(&dll, b"not a PE").expect("write dll fixture");

        let manifest = GameManifest {
            root: root.clone(),
            files: vec![FileRecord {
                path: PathBuf::from("renderer.dll"),
                size: 9,
                sha256: String::new(),
                extension: Some("dll".to_owned()),
            }],
            executables: vec![ExecutableRecord {
                path: PathBuf::from("game.exe"),
                format: "PE".to_owned(),
                architecture: Some("x86_64".to_owned()),
                protection: ProtectionSignals::default(),
            }],
        };

        assert!(profile_game(&manifest).is_err());
        std::fs::remove_file(exe).expect("remove exe fixture");
        std::fs::remove_file(dll).expect("remove dll fixture");
        std::fs::remove_dir(root).expect("remove fixture dir");
    }
}
