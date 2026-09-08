use duxo_analyzer::{BinaryDependency, GameProfile};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdaptationPlan {
    pub steps: Vec<AdaptationStep>,
    pub required_modules: Vec<String>,
    pub dependency_resolutions: Vec<DependencyResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdaptationStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub module: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Planned,
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyResolution {
    pub dependency: BinaryDependency,
    pub kind: DependencyResolutionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyResolutionKind {
    BundledGame,
    WindowsPlatform,
    KnownRuntime,
    Unresolved,
}

pub fn build_plan(profile: &GameProfile) -> AdaptationPlan {
    let mut steps = Vec::new();
    let mut modules = Vec::new();
    let dependency_resolutions = resolve_dependencies(profile);

    add_step(
        &mut steps,
        &mut modules,
        "analyze",
        "Analyze game",
        "Review the detected PE binaries, graphics, API, runtime, dependency, and protection requirements.",
        "core.analyzer",
        StepStatus::Ready,
    );

    if profile.graphics.direct3d9 {
        add_translation_step(
            &mut steps,
            &mut modules,
            "graphics-d3d9",
            "Prepare Direct3D 9 → Metal",
            "A D3D9 conversion executor is not registered yet, so Duxo will not claim this step is executable.",
            "graphics.d3d9",
        );
    }
    if profile.graphics.direct3d10 {
        add_translation_step(
            &mut steps,
            &mut modules,
            "graphics-d3d10",
            "Prepare Direct3D 10 → Metal",
            "A D3D10 conversion executor is not registered yet, so Duxo will not claim this step is executable.",
            "graphics.d3d10",
        );
    }
    if profile.graphics.direct3d11 {
        add_translation_step(
            &mut steps,
            &mut modules,
            "graphics-d3d11",
            "Prepare Direct3D 11 → Metal",
            "A D3D11 conversion executor is not registered yet, so Duxo will not claim this step is executable.",
            "graphics.d3d11",
        );
    }
    if profile.graphics.direct3d12 {
        add_translation_step(
            &mut steps,
            &mut modules,
            "graphics-d3d12",
            "Prepare Direct3D 12 → Metal",
            "A D3D12 conversion executor is not registered yet, so Duxo will not claim this step is executable.",
            "graphics.d3d12",
        );
    }
    if profile.graphics.dxgi {
        add_translation_step(
            &mut steps,
            &mut modules,
            "graphics-dxgi",
            "Map DXGI requirements",
            "A DXGI adaptation executor is not registered yet; this requirement remains blocked rather than being reported as completed.",
            "graphics.dxgi",
        );
    }
    if profile.graphics.vulkan {
        add_translation_step(
            &mut steps,
            &mut modules,
            "graphics-vulkan",
            "Prepare Vulkan path",
            "A Vulkan adaptation executor is not registered yet; Duxo only records the detected requirement.",
            "graphics.vulkan",
        );
    }
    if profile.graphics.opengl {
        add_translation_step(
            &mut steps,
            &mut modules,
            "graphics-opengl",
            "Prepare OpenGL path",
            "An OpenGL adaptation executor is not registered yet; Duxo only records the detected requirement.",
            "graphics.opengl",
        );
    }

    for api in &profile.windows_apis {
        let id = format!("api-{}", slug(&api.family));
        add_translation_step(
            &mut steps,
            &mut modules,
            &id,
            &format!("Map {} API family", api.family),
            "A native implementation for this Windows API family is not registered yet; the plan records the requirement without pretending it is ready.",
            &format!("windows-api.{}", slug(&api.family)),
        );
    }
    for runtime in &profile.runtimes {
        let id = format!("runtime-{}", slug(&runtime.name));
        add_translation_step(
            &mut steps,
            &mut modules,
            &id,
            &format!("Prepare {}", runtime.name),
            "A runtime preparation executor is not registered yet; the requirement remains blocked until one exists.",
            &format!("runtime.{}", slug(&runtime.name)),
        );
    }

    if dependency_resolutions
        .iter()
        .any(|r| r.kind == DependencyResolutionKind::Unresolved)
    {
        add_step(
            &mut steps,
            &mut modules,
            "resolve-dependencies",
            "Resolve game dependencies",
            "The analyzer found imports that are not present in the supplied game files and are not known platform/runtime dependencies. These must be resolved before a build can be considered complete.",
            "core.dependencies",
            StepStatus::Blocked,
        );
    } else if !dependency_resolutions.is_empty() {
        add_step(
            &mut steps,
            &mut modules,
            "resolve-dependencies",
            "Resolve game dependencies",
            "All detected binary imports have an explicit bundled-game, Windows-platform, or known-runtime resolution.",
            "core.dependencies",
            StepStatus::Ready,
        );
    }

    if !profile.protections.packers_or_protectors.is_empty() {
        add_step(
            &mut steps,
            &mut modules,
            "protection-review",
            "Review binary protection signals",
            "Record protection signals for planning and validation. No protection is modified or bypassed by this planning stage.",
            "analysis.protection",
            StepStatus::Ready,
        );
    }
    if !profile.protections.anti_cheats.is_empty() {
        add_step(
            &mut steps,
            &mut modules,
            "anti-cheat-review",
            "Review anti-cheat signals",
            "Record detected anti-cheat signals so the conversion plan can account for the title's requirements.",
            "analysis.anti_cheat",
            StepStatus::Ready,
        );
    }

    add_step(
        &mut steps,
        &mut modules,
        "build",
        "Build target application",
        "The target application builder is not executable yet, so Duxo must not report a generated application.",
        "core.builder",
        StepStatus::Blocked,
    );
    add_step(
        &mut steps,
        &mut modules,
        "validate",
        "Validate generated application",
        "Launch/package validation cannot run until an executable build exists.",
        "core.validator",
        StepStatus::Blocked,
    );

    modules.sort();
    modules.dedup();
    AdaptationPlan {
        steps,
        required_modules: modules,
        dependency_resolutions,
    }
}

fn resolve_dependencies(profile: &GameProfile) -> Vec<DependencyResolution> {
    profile
        .dependencies
        .iter()
        .map(|dependency| {
            let kind = if profile.unresolved_imports.contains(dependency) {
                DependencyResolutionKind::Unresolved
            } else if is_known_runtime(&dependency.library) {
                DependencyResolutionKind::KnownRuntime
            } else if is_windows_platform(&dependency.library) {
                DependencyResolutionKind::WindowsPlatform
            } else {
                DependencyResolutionKind::BundledGame
            };
            DependencyResolution {
                dependency: dependency.clone(),
                kind,
            }
        })
        .collect()
}

fn is_known_runtime(library: &str) -> bool {
    matches!(
        library,
        "vcruntime140.dll"
            | "vcruntime140_1.dll"
            | "msvcp140.dll"
            | "ucrtbase.dll"
            | "msvcp120.dll"
            | "msvcr120.dll"
            | "msvcp110.dll"
            | "msvcr110.dll"
            | "mscoree.dll"
    )
}

fn is_windows_platform(library: &str) -> bool {
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
    )
}

fn add_translation_step(
    steps: &mut Vec<AdaptationStep>,
    modules: &mut Vec<String>,
    id: &str,
    title: &str,
    description: &str,
    module: &str,
) {
    add_step(
        steps,
        modules,
        id,
        title,
        description,
        module,
        StepStatus::Blocked,
    );
}

fn add_step(
    steps: &mut Vec<AdaptationStep>,
    modules: &mut Vec<String>,
    id: &str,
    title: &str,
    description: &str,
    module: &str,
    status: StepStatus,
) {
    modules.push(module.to_owned());
    steps.push(AdaptationStep {
        id: id.to_owned(),
        title: title.to_owned(),
        description: description.to_owned(),
        module: module.to_owned(),
        status,
    });
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{DependencyResolutionKind, StepStatus, build_plan};
    use duxo_analyzer::profile::{
        BinaryDependency, ExecutableProfile, GameProfile, GraphicsRequirements, ProtectionSummary,
    };

    fn base_profile() -> GameProfile {
        GameProfile {
            executables: vec![ExecutableProfile {
                path: "game.exe".to_owned(),
                architecture: Some("x86_64".to_owned()),
                format: "PE".to_owned(),
            }],
            pe_binaries: Vec::new(),
            dependencies: Vec::new(),
            graphics: GraphicsRequirements::default(),
            windows_apis: Vec::new(),
            runtimes: Vec::new(),
            unresolved_imports: Vec::new(),
            protections: ProtectionSummary::default(),
        }
    }

    #[test]
    fn creates_game_specific_graphics_steps_without_faking_readiness() {
        let mut profile = base_profile();
        profile.graphics.direct3d11 = true;
        profile.graphics.dxgi = true;
        let plan = build_plan(&profile);
        assert!(
            plan.steps
                .iter()
                .any(|s| s.id == "graphics-d3d11" && s.status == StepStatus::Blocked)
        );
        assert!(
            plan.steps
                .iter()
                .any(|s| s.id == "graphics-dxgi" && s.status == StepStatus::Blocked)
        );
        assert!(
            plan.steps
                .iter()
                .any(|s| s.id == "validate" && s.status == StepStatus::Blocked)
        );
    }

    #[test]
    fn resolves_dependency_classes_explicitly() {
        let mut profile = base_profile();
        profile.dependencies = vec![
            BinaryDependency {
                importer: "game.exe".to_owned(),
                library: "renderer.dll".to_owned(),
            },
            BinaryDependency {
                importer: "game.exe".to_owned(),
                library: "kernel32.dll".to_owned(),
            },
            BinaryDependency {
                importer: "game.exe".to_owned(),
                library: "vcruntime140.dll".to_owned(),
            },
            BinaryDependency {
                importer: "game.exe".to_owned(),
                library: "missing.dll".to_owned(),
            },
        ];
        profile.unresolved_imports = vec![BinaryDependency {
            importer: "game.exe".to_owned(),
            library: "missing.dll".to_owned(),
        }];
        let plan = build_plan(&profile);
        assert_eq!(plan.dependency_resolutions.len(), 4);
        assert_eq!(
            plan.dependency_resolutions[0].kind,
            DependencyResolutionKind::BundledGame
        );
        assert_eq!(
            plan.dependency_resolutions[1].kind,
            DependencyResolutionKind::WindowsPlatform
        );
        assert_eq!(
            plan.dependency_resolutions[2].kind,
            DependencyResolutionKind::KnownRuntime
        );
        assert_eq!(
            plan.dependency_resolutions[3].kind,
            DependencyResolutionKind::Unresolved
        );
        assert!(plan.steps.iter().any(|step| {
            step.id == "resolve-dependencies" && step.status == StepStatus::Blocked
        }));
    }

    #[test]
    fn resolved_dependencies_make_dependency_step_ready() {
        let mut profile = base_profile();
        profile.dependencies = vec![
            BinaryDependency {
                importer: "game.exe".to_owned(),
                library: "renderer.dll".to_owned(),
            },
            BinaryDependency {
                importer: "game.exe".to_owned(),
                library: "kernel32.dll".to_owned(),
            },
        ];
        let plan = build_plan(&profile);
        assert!(
            plan.steps
                .iter()
                .any(|step| step.id == "resolve-dependencies" && step.status == StepStatus::Ready)
        );
    }

    #[test]
    fn unresolved_imports_block_dependency_resolution() {
        let mut profile = base_profile();
        profile.unresolved_imports = vec![BinaryDependency {
            importer: "game.exe".to_owned(),
            library: "renderer.dll".to_owned(),
        }];
        profile.dependencies = profile.unresolved_imports.clone();
        let plan = build_plan(&profile);
        assert!(
            plan.steps
                .iter()
                .any(|s| s.id == "resolve-dependencies" && s.status == StepStatus::Blocked)
        );
    }
}
