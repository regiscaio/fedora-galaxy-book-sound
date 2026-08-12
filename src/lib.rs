mod i18n;
mod updates;

use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use i18n::{init_i18n, tr, tr_mark, trf, trn};
pub use updates::{install_package_updates, package_update_names};

pub const APP_ID: &str = "com.caioregis.GalaxyBookSound";
pub const APP_NAME: &str = "Galaxy Book Sound";
pub const EQUALIZER_FREQUENCIES_HZ: [u32; 10] =
    [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

const PIPEWIRE_FILTER_DESCRIPTION: &str = "Galaxy Book Sound";
const PIPEWIRE_FILTER_MAIN_NODE: &str = "galaxybook_sound_sink";
const PIPEWIRE_FILTER_STREAM_NODE: &str = "galaxybook_sound_output";
const PIPEWIRE_FILTER_NAME: &str = "galaxybook-sound";
const PIPEWIRE_COMBINED_SINK_NODE: &str = "galaxybook_sound_combined_sink";
const PIPEWIRE_COMBINED_SINK_DESCRIPTION: &str = "Galaxy Book Sound - Saída combinada";
const PIPEWIRE_FILTER_CONFIG_NAME: &str = "99-galaxybook-sound.conf";
const RESTART_AUDIO_STACK_COMMAND: &str =
    "systemctl --user restart wireplumber.service pipewire.service pipewire-pulse.service";
const SESSION_RETRY_ATTEMPTS: usize = 60;
const SESSION_RETRY_DELAY: Duration = Duration::from_millis(250);
const STATE_QUERY_ATTEMPTS: usize = 3;
const STATE_QUERY_RETRY_DELAY: Duration = Duration::from_millis(100);
const TARGET_REVALIDATION_ATTEMPTS: usize = 2;
const ATMOS_OVERLAY_DB: [f64; 10] = [0.8, 1.1, 0.9, 0.5, 0.2, 0.0, 0.6, 0.9, 0.8, 0.6];
const ATMOS_WIDTH_GAIN: f64 = 1.08;
const ATMOS_CROSSFEED_GAIN: f64 = -0.08;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioProfile {
    Flat,
    Music,
    Cinema,
    AtmosCompatible,
}

impl AudioProfile {
    pub fn selectable() -> &'static [Self] {
        const PROFILES: [AudioProfile; 3] = [
            AudioProfile::Flat,
            AudioProfile::Music,
            AudioProfile::Cinema,
        ];
        &PROFILES
    }

    fn normalized(self) -> Self {
        match self {
            Self::AtmosCompatible => Self::Cinema,
            other => other,
        }
    }

    pub fn title(self) -> &'static str {
        match self.normalized() {
            Self::Flat => tr_mark("Neutro"),
            Self::Music => tr_mark("Música"),
            Self::Cinema => tr_mark("Cinema"),
            Self::AtmosCompatible => unreachable!(),
        }
    }

    pub fn preset_name(self) -> &'static str {
        match self.normalized() {
            Self::Flat => "Galaxy Book Sound - Neutro",
            Self::Music => "Galaxy Book Sound - Música",
            Self::Cinema => "Galaxy Book Sound - Cinema",
            Self::AtmosCompatible => unreachable!(),
        }
    }

    pub fn selected_index(self) -> u32 {
        match self.normalized() {
            Self::Flat => 0,
            Self::Music => 1,
            Self::Cinema => 2,
            Self::AtmosCompatible => unreachable!(),
        }
    }

    pub fn from_selected_index(index: u32) -> Self {
        Self::selectable()
            .get(index as usize)
            .copied()
            .unwrap_or_default()
    }
}

impl Default for AudioProfile {
    fn default() -> Self {
        Self::Flat
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoundAppConfig {
    pub selected_profile: AudioProfile,
    #[serde(default)]
    pub atmos_enabled: bool,
    #[serde(default)]
    pub combined_output_enabled: bool,
    pub bands_db: [f64; 10],
}

impl Default for SoundAppConfig {
    fn default() -> Self {
        let selected_profile = AudioProfile::default();
        Self {
            selected_profile,
            atmos_enabled: false,
            combined_output_enabled: false,
            bands_db: default_bands_for_profile(selected_profile),
        }
    }
}

impl SoundAppConfig {
    pub fn normalize(&mut self) {
        if self.selected_profile == AudioProfile::AtmosCompatible {
            self.selected_profile = AudioProfile::Cinema;
            self.atmos_enabled = true;
        }

        for gain in &mut self.bands_db {
            *gain = clamp_gain(*gain);
        }
    }

    pub fn preset_name(&self) -> String {
        let base = self.selected_profile.preset_name();
        if self.atmos_enabled {
            format!("{base} + Atmos compatível")
        } else {
            base.to_string()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SoundSessionState {
    pub pipewire_available: bool,
    pub config_present: bool,
    pub filter_nodes_present: bool,
    pub physical_output_present: bool,
    pub target_linked: bool,
    pub filter_active: bool,
    pub combined_output_active: bool,
    pub active_profile_name: Option<String>,
    pub target_output: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PipeWireNode {
    id: u32,
    node_name: Option<String>,
    node_description: Option<String>,
    node_nick: Option<String>,
    media_class: Option<String>,
    device_profile_description: Option<String>,
    node_link_group: Option<String>,
    object_path: Option<String>,
    device_id: Option<u32>,
    node_virtual: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PipeWireLink {
    output_node_id: u32,
    input_node_id: u32,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PipeWireGraph {
    nodes: Vec<PipeWireNode>,
    links: Vec<PipeWireLink>,
}

#[derive(Debug, Deserialize)]
struct PipeWireDumpObject {
    id: u32,
    #[serde(rename = "type")]
    object_type: String,
    info: Option<PipeWireDumpInfo>,
}

#[derive(Debug, Default, Deserialize)]
struct PipeWireDumpInfo {
    #[serde(default)]
    props: HashMap<String, Value>,
    #[serde(default, rename = "output-node-id")]
    output_node_id: Option<u32>,
    #[serde(default, rename = "input-node-id")]
    input_node_id: Option<u32>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct GraphProfileState {
    filter_nodes_present: bool,
    physical_output_present: bool,
    target_linked: bool,
    filter_active: bool,
    combined_output_active: bool,
    target_output: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProfileActivationWaitError {
    TargetChanged,
    Timeout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetQueryError {
    SessionUnavailable,
    PhysicalOutputUnavailable,
}

pub fn default_bands_for_profile(profile: AudioProfile) -> [f64; 10] {
    match profile.normalized() {
        AudioProfile::Flat => [0.0; 10],
        AudioProfile::Music => [1.0, 2.0, 1.8, 0.8, 0.2, 0.0, 0.8, 1.4, 1.0, 0.6],
        AudioProfile::Cinema => [1.8, 2.8, 2.2, 1.0, 0.3, 0.1, 1.0, 1.8, 1.2, 0.8],
        AudioProfile::AtmosCompatible => unreachable!(),
    }
}

pub fn load_sound_app_config() -> SoundAppConfig {
    let path = sound_app_config_path();
    let Ok(text) = fs::read_to_string(path) else {
        return SoundAppConfig::default();
    };
    serde_json::from_str::<SoundAppConfig>(&text)
        .map(|mut config| {
            config.normalize();
            config
        })
        .unwrap_or_else(|_| SoundAppConfig::default())
}

pub fn save_sound_app_config(config: &SoundAppConfig) -> Result<(), String> {
    let path = sound_app_config_path();
    let json = serde_json::to_string_pretty(config).map_err(|error| {
        trf(
            "Falha ao serializar a configuração de áudio: {error}",
            &[("error", error.to_string())],
        )
    })?;
    atomic_write(&path, json.as_bytes()).map_err(|error| {
        trf(
            "Falha ao salvar a configuração de áudio: {error}",
            &[("error", error.to_string())],
        )
    })
}

pub fn save_and_apply_profile(config: &SoundAppConfig) -> Result<String, String> {
    let mut normalized_config = config.clone();
    normalized_config.normalize();

    let pipewire_config_path = pipewire_filter_config_path();
    let previous_pipewire_config = read_optional_file(&pipewire_config_path).map_err(|error| {
        trf(
            "Falha ao ler a configuração anterior do PipeWire: {error}",
            &[("error", error.to_string())],
        )
    })?;
    let mut configuration_replaced = false;

    for revalidation in 0..=TARGET_REVALIDATION_ATTEMPTS {
        let (_graph, physical_target) = match pipewire_graph_and_target_with_retry(
            SESSION_RETRY_ATTEMPTS,
            SESSION_RETRY_DELAY,
        ) {
            Ok(result) => result,
            Err(TargetQueryError::SessionUnavailable) if !configuration_replaced => {
                return Err(tr(
                    "Não foi possível consultar a sessão do PipeWire antes de aplicar o pipeline.",
                ));
            }
            Err(TargetQueryError::PhysicalOutputUnavailable) if !configuration_replaced => {
                return Err(tr(
                    "Não foi possível identificar a saída física interna do notebook para aplicar o pipeline.",
                ));
            }
            Err(TargetQueryError::SessionUnavailable) => {
                return Err(rollback_after_apply_failure(
                    &pipewire_config_path,
                    previous_pipewire_config.as_deref(),
                    tr(
                        "A sessão do PipeWire deixou de responder durante a revalidação da saída física.",
                    ),
                ));
            }
            Err(TargetQueryError::PhysicalOutputUnavailable) => {
                return Err(rollback_after_apply_failure(
                    &pipewire_config_path,
                    previous_pipewire_config.as_deref(),
                    tr("A saída física interna desapareceu durante a revalidação do pipeline."),
                ));
            }
        };
        let content = match validated_pipewire_filter_config(&normalized_config, &physical_target) {
            Ok(content) => content,
            Err(error) if !configuration_replaced => return Err(error),
            Err(error) => {
                return Err(rollback_after_apply_failure(
                    &pipewire_config_path,
                    previous_pipewire_config.as_deref(),
                    error,
                ));
            }
        };

        if let Err(error) = atomic_write(&pipewire_config_path, content.as_bytes()) {
            let error = trf(
                "Falha ao substituir a configuração do PipeWire: {error}",
                &[("error", error.to_string())],
            );
            return if configuration_replaced {
                Err(rollback_after_apply_failure(
                    &pipewire_config_path,
                    previous_pipewire_config.as_deref(),
                    error,
                ))
            } else {
                Err(error)
            };
        }
        configuration_replaced = true;

        if let Err(error) = restart_audio_stack() {
            return Err(rollback_after_apply_failure(
                &pipewire_config_path,
                previous_pipewire_config.as_deref(),
                error,
            ));
        }

        match wait_for_profile_activation(&normalized_config, &physical_target) {
            Ok(()) => {
                if let Err(error) = save_sound_app_config(&normalized_config) {
                    return Err(rollback_after_apply_failure(
                        &pipewire_config_path,
                        previous_pipewire_config.as_deref(),
                        error,
                    ));
                }
                return Ok(normalized_config.preset_name());
            }
            Err(ProfileActivationWaitError::TargetChanged)
                if revalidation < TARGET_REVALIDATION_ATTEMPTS =>
            {
                continue;
            }
            Err(ProfileActivationWaitError::TargetChanged) => {
                return Err(rollback_after_apply_failure(
                    &pipewire_config_path,
                    previous_pipewire_config.as_deref(),
                    tr("A saída física interna mudou repetidamente durante a aplicação do perfil."),
                ));
            }
            Err(ProfileActivationWaitError::Timeout) => {
                let error = if normalized_config.combined_output_enabled {
                    tr(
                        "O PipeWire não confirmou o filtro, a saída combinada e o vínculo com a saída física após aplicar a configuração.",
                    )
                } else {
                    tr(
                        "O PipeWire não confirmou o filtro e o vínculo com a saída física após aplicar a configuração.",
                    )
                };
                return Err(rollback_after_apply_failure(
                    &pipewire_config_path,
                    previous_pipewire_config.as_deref(),
                    error,
                ));
            }
        }
    }

    unreachable!("the target revalidation loop always returns")
}

pub fn collect_sound_session_state() -> SoundSessionState {
    let config_present = pipewire_filter_config_path().exists();
    let Ok(graph) = pipewire_graph_with_retry(STATE_QUERY_ATTEMPTS, STATE_QUERY_RETRY_DELAY) else {
        return SoundSessionState {
            pipewire_available: false,
            config_present,
            ..SoundSessionState::default()
        };
    };

    let saved_config = load_sound_app_config();
    let graph_state = evaluate_profile_graph(&graph, &saved_config, None);
    let profile_active = graph_state.filter_active
        && (!saved_config.combined_output_enabled || graph_state.combined_output_active);
    let active_profile_name = if profile_active {
        Some(saved_config.preset_name())
    } else {
        None
    };

    SoundSessionState {
        pipewire_available: true,
        config_present,
        filter_nodes_present: graph_state.filter_nodes_present,
        physical_output_present: graph_state.physical_output_present,
        target_linked: graph_state.target_linked,
        filter_active: graph_state.filter_active,
        combined_output_active: graph_state.combined_output_active,
        active_profile_name,
        target_output: graph_state.target_output,
    }
}

pub fn run_smoke_test() -> Result<(), String> {
    let config = load_sound_app_config();
    let target = PipeWireNode {
        id: 1,
        node_name: Some(
            "alsa_output.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__Speaker__sink".into(),
        ),
        node_description: Some("Meteor Lake-P HD Audio Controller Speaker".into()),
        media_class: Some("Audio/Sink".into()),
        device_profile_description: Some("Speaker".into()),
        device_id: Some(1),
        ..PipeWireNode::default()
    };
    let conf_text = validated_pipewire_filter_config(&config, &target)?;
    if !conf_text.contains(PIPEWIRE_FILTER_MAIN_NODE) || !conf_text.contains("filter.smart = true")
    {
        return Err(tr("Falha ao gerar a configuração do PipeWire do app."));
    }
    Ok(())
}

fn effective_bands_for_config(config: &SoundAppConfig) -> [f64; 10] {
    let mut effective = config.bands_db;

    if config.atmos_enabled {
        for (index, gain) in effective.iter_mut().enumerate() {
            *gain = clamp_gain(*gain + ATMOS_OVERLAY_DB[index]);
        }
    }

    effective
}

fn validated_pipewire_filter_config(
    config: &SoundAppConfig,
    target: &PipeWireNode,
) -> Result<String, String> {
    let target_name = target
        .node_name
        .as_deref()
        .ok_or_else(|| tr("A saída alvo do notebook não expôs um node.name válido no PipeWire."))?;
    if target_name.trim().is_empty() {
        return Err(tr(
            "A saída alvo do notebook expôs um node.name vazio no PipeWire.",
        ));
    }
    if !is_internal_speaker_output(target) {
        return Err(tr(
            "A saída alvo não foi confirmada como a saída física interna do notebook.",
        ));
    }

    Ok(build_pipewire_filter_config(
        config,
        &PipeWireNode {
            node_name: Some(target_name.to_string()),
            node_description: target.node_description.clone(),
            media_class: target.media_class.clone(),
            device_profile_description: target.device_profile_description.clone(),
            ..PipeWireNode::default()
        },
    ))
}

fn read_optional_file(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "configuration path has no parent directory",
        )
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "configuration path has no file name",
        )
    })?;
    fs::create_dir_all(parent)?;

    for attempt in 0..128_u32 {
        let mut temporary_name = OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".{}.{}.tmp", std::process::id(), attempt));
        let temporary_path = parent.join(temporary_name);
        let mut temporary_file = match OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temporary_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };

        if let Err(error) = temporary_file
            .write_all(contents)
            .and_then(|()| temporary_file.sync_all())
        {
            drop(temporary_file);
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }
        drop(temporary_file);

        if let Err(error) = fs::rename(&temporary_path, path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }

        // The rename already made the replacement atomic. Directory sync is
        // best-effort because some user configuration filesystems reject it.
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique temporary configuration file",
    ))
}

fn restore_optional_file(path: &Path, previous_contents: Option<&[u8]>) -> io::Result<()> {
    match previous_contents {
        Some(contents) => atomic_write(path, contents),
        None => match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        },
    }
}

fn rollback_after_apply_failure(
    pipewire_config_path: &Path,
    previous_pipewire_config: Option<&[u8]>,
    primary_error: String,
) -> String {
    if let Err(error) = restore_optional_file(pipewire_config_path, previous_pipewire_config) {
        return trf(
            "{primary}\n\nTambém não foi possível restaurar a configuração anterior do PipeWire: {error}",
            &[("primary", primary_error), ("error", error.to_string())],
        );
    }

    match restart_audio_stack() {
        Ok(()) => trf(
            "{primary}\n\nA configuração anterior do PipeWire foi restaurada e reativada.",
            &[("primary", primary_error)],
        ),
        Err(error) => trf(
            "{primary}\n\nA configuração anterior do PipeWire foi restaurada, mas a sessão de áudio não pôde ser reativada: {error}",
            &[("primary", primary_error), ("error", error)],
        ),
    }
}

fn build_pipewire_filter_config(config: &SoundAppConfig, target: &PipeWireNode) -> String {
    let physical_target_name = target.node_name.as_deref().unwrap_or_default();
    let smart_target_name = if config.combined_output_enabled {
        PIPEWIRE_COMBINED_SINK_NODE
    } else {
        physical_target_name
    };
    let target_name = escape_spa_string(smart_target_name);
    let target_label = escape_spa_string(
        target
            .node_description
            .as_deref()
            .or(target.node_name.as_deref())
            .unwrap_or_default(),
    );
    let graph = build_filter_graph(config);
    let combined_output_module = if config.combined_output_enabled {
        build_combined_output_module(physical_target_name)
    } else {
        String::new()
    };

    format!(
        "# Generated by {APP_NAME}. Changes will be overwritten.\n\
         # Target output: {target_label}\n\
         context.modules = [\n\
         {combined_output_module}\
         \t{{ name = libpipewire-module-filter-chain\n\
         \t    flags = [ nofail ]\n\
         \t    args = {{\n\
         \t        node.description = \"{PIPEWIRE_FILTER_DESCRIPTION}\"\n\
         \t        media.name = \"{PIPEWIRE_FILTER_DESCRIPTION}\"\n\
         {graph}\
         \t        capture.props = {{\n\
         \t            node.name = \"{PIPEWIRE_FILTER_MAIN_NODE}\"\n\
         \t            audio.position = [ FL FR ]\n\
         \t            media.class = Audio/Sink\n\
         \t            filter.smart = true\n\
         \t            filter.smart.name = \"{PIPEWIRE_FILTER_NAME}\"\n\
         \t            filter.smart.target = {{ node.name = \"{target_name}\" }}\n\
         \t        }}\n\
         \t        playback.props = {{\n\
         \t            node.name = \"{PIPEWIRE_FILTER_STREAM_NODE}\"\n\
         \t            audio.position = [ FL FR ]\n\
         \t            node.passive = true\n\
         \t            stream.dont-remix = true\n\
         \t        }}\n\
         \t    }}\n\
         \t}}\n\
         ]\n"
    )
}

fn build_combined_output_module(physical_target_name: &str) -> String {
    let combined_description = escape_spa_string(PIPEWIRE_COMBINED_SINK_DESCRIPTION);
    let physical_target_name = escape_spa_string(physical_target_name);

    format!(
        "\t{{ name = libpipewire-module-combine-stream\n\
         \t    flags = [ nofail ]\n\
         \t    args = {{\n\
         \t        combine.mode = sink\n\
         \t        node.name = \"{PIPEWIRE_COMBINED_SINK_NODE}\"\n\
         \t        node.description = \"{combined_description}\"\n\
         \t        combine.latency-compensate = true\n\
         \t        combine.props = {{\n\
         \t            audio.position = [ FL FR ]\n\
         \t            media.class = Audio/Sink\n\
         \t            node.virtual = true\n\
         \t        }}\n\
         \t        stream.rules = [\n\
         \t            {{\n\
         \t                matches = [\n\
         \t                    {{\n\
         \t                        media.class = \"Audio/Sink\"\n\
         \t                        node.name = \"{physical_target_name}\"\n\
         \t                    }}\n\
         \t                ]\n\
         \t                actions = {{\n\
         \t                    create-stream = {{\n\
         \t                        audio.position = [ FL FR ]\n\
         \t                        combine.audio.position = [ FL FR ]\n\
         \t                    }}\n\
         \t                }}\n\
         \t            }}\n\
         \t        ]\n\
         \t    }}\n\
         \t}}\n"
    )
}

fn build_filter_graph(config: &SoundAppConfig) -> String {
    let filters = effective_bands_for_config(config);
    let filter_lines = EQUALIZER_FREQUENCIES_HZ
        .iter()
        .zip(filters.iter())
        .map(|(frequency_hz, gain_db)| {
            format!(
                "\t\t\t\t\t{{ type = bq_peaking freq = {:.1} gain = {:.2} q = 1.0 }}",
                *frequency_hz as f64, gain_db
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let (extra_nodes, extra_links, outputs) = if config.atmos_enabled {
        (
            format!(
                "\t\t\t\t{{\n\
                 \t\t\t\t\ttype = builtin\n\
                 \t\t\t\t\tname = widen_l\n\
                 \t\t\t\t\tlabel = mixer\n\
                 \t\t\t\t\tcontrol = {{ \"Gain 1\" = {ATMOS_WIDTH_GAIN:.2} \"Gain 2\" = {ATMOS_CROSSFEED_GAIN:.2} }}\n\
                 \t\t\t\t}}\n\
                 \t\t\t\t{{\n\
                 \t\t\t\t\ttype = builtin\n\
                 \t\t\t\t\tname = widen_r\n\
                 \t\t\t\t\tlabel = mixer\n\
                 \t\t\t\t\tcontrol = {{ \"Gain 1\" = {ATMOS_CROSSFEED_GAIN:.2} \"Gain 2\" = {ATMOS_WIDTH_GAIN:.2} }}\n\
                 \t\t\t\t}}\n"
            ),
            "\t\t\t\t{ output = \"eq:Out 1\" input = \"widen_l:In 1\" }\n\
             \t\t\t\t{ output = \"eq:Out 2\" input = \"widen_l:In 2\" }\n\
             \t\t\t\t{ output = \"eq:Out 1\" input = \"widen_r:In 1\" }\n\
             \t\t\t\t{ output = \"eq:Out 2\" input = \"widen_r:In 2\" }\n"
                .to_string(),
            "\t\t\t\t\"widen_l:Out\"\n\t\t\t\t\"widen_r:Out\"".to_string(),
        )
    } else {
        (
            String::new(),
            String::new(),
            "\t\t\t\t\"eq:Out 1\"\n\t\t\t\t\"eq:Out 2\"".to_string(),
        )
    };

    format!(
        "\t        filter.graph = {{\n\
         \t            nodes = [\n\
         \t\t\t\t{{\n\
         \t\t\t\t\ttype = builtin\n\
         \t\t\t\t\tname = eq\n\
         \t\t\t\t\tlabel = param_eq\n\
         \t\t\t\t\tconfig = {{\n\
         \t\t\t\t\t\tfilters = [\n\
         {filter_lines}\n\
         \t\t\t\t\t\t]\n\
         \t\t\t\t\t}}\n\
         \t\t\t\t}}\n\
         {extra_nodes}\
         \t            ]\n\
         \t            links = [\n\
         {extra_links}\
         \t            ]\n\
         \t            inputs = [\n\
         \t\t\t\t\"eq:In 1\"\n\
         \t\t\t\t\"eq:In 2\"\n\
         \t            ]\n\
         \t            outputs = [\n\
         {outputs}\n\
         \t            ]\n\
         \t        }}\n"
    )
}

fn restart_audio_stack() -> Result<(), String> {
    if !systemd_available() {
        return Err(tr(
            "Systemd de usuário não está disponível. Não foi possível reiniciar PipeWire e WirePlumber.",
        ));
    }

    let output = Command::new("bash")
        .args(["-lc", RESTART_AUDIO_STACK_COMMAND])
        .output()
        .map_err(|error| {
            trf(
                "Falha ao reiniciar a sessão de áudio: {error}",
                &[("error", error.to_string())],
            )
        })?;

    if output.status.success() {
        wait_for_pipewire_session()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            tr("A sessão de áudio não aceitou o reinício do PipeWire e do WirePlumber.")
        })
    }
}

fn wait_for_pipewire_session() -> Result<(), String> {
    for attempt in 0..SESSION_RETRY_ATTEMPTS {
        if pipewire_graph().is_ok() {
            return Ok(());
        }
        if attempt + 1 < SESSION_RETRY_ATTEMPTS {
            thread::sleep(SESSION_RETRY_DELAY);
        }
    }

    Err(tr(
        "O PipeWire não voltou a responder após o reinício da sessão de áudio.",
    ))
}

fn wait_for_profile_activation(
    config: &SoundAppConfig,
    physical_target: &PipeWireNode,
) -> Result<(), ProfileActivationWaitError> {
    let expected_target_name = physical_target.node_name.as_deref().unwrap_or_default();

    for attempt in 0..SESSION_RETRY_ATTEMPTS {
        if let Ok(graph) = pipewire_graph() {
            let expected_target_present = graph.nodes.iter().any(|node| {
                node.node_name.as_deref() == Some(expected_target_name)
                    && is_internal_speaker_output(node)
            });
            if !expected_target_present {
                if let Some(new_target) = detect_target_sink_from_graph(&graph) {
                    if new_target.node_name.as_deref() != Some(expected_target_name) {
                        return Err(ProfileActivationWaitError::TargetChanged);
                    }
                }
            }

            let state = evaluate_profile_graph(&graph, config, Some(expected_target_name));
            if state.filter_active
                && state.target_linked
                && (!config.combined_output_enabled || state.combined_output_active)
            {
                return Ok(());
            }
        }
        if attempt + 1 < SESSION_RETRY_ATTEMPTS {
            thread::sleep(SESSION_RETRY_DELAY);
        }
    }

    Err(ProfileActivationWaitError::Timeout)
}

fn evaluate_profile_graph(
    graph: &PipeWireGraph,
    config: &SoundAppConfig,
    expected_physical_target: Option<&str>,
) -> GraphProfileState {
    let filter_main = graph.nodes.iter().find(|node| {
        node.node_name.as_deref() == Some(PIPEWIRE_FILTER_MAIN_NODE)
            && node.media_class.as_deref() == Some("Audio/Sink")
    });
    let filter_stream = graph.nodes.iter().find(|node| {
        node.node_name.as_deref() == Some(PIPEWIRE_FILTER_STREAM_NODE)
            && node.media_class.as_deref() == Some("Stream/Output/Audio")
    });
    let filter_nodes_present = filter_main.is_some() || filter_stream.is_some();
    let valid_filter_pair = filter_main
        .zip(filter_stream)
        .is_some_and(|(main, stream)| {
            match (
                main.node_link_group.as_deref(),
                stream.node_link_group.as_deref(),
            ) {
                (Some(main_group), Some(stream_group)) => {
                    !main_group.is_empty() && main_group == stream_group
                }
                _ => false,
            }
        });

    let physical_target = if let Some(expected_name) = expected_physical_target {
        graph.nodes.iter().find(|node| {
            node.node_name.as_deref() == Some(expected_name) && is_internal_speaker_output(node)
        })
    } else {
        graph
            .nodes
            .iter()
            .find(|node| is_internal_speaker_output(node))
    };
    let physical_output_present = physical_target.is_some();
    let target_output = physical_target.and_then(node_display_name);

    let Some((_filter_main, filter_stream)) = filter_main.zip(filter_stream) else {
        return GraphProfileState {
            filter_nodes_present,
            physical_output_present,
            target_output,
            ..GraphProfileState::default()
        };
    };
    if !valid_filter_pair {
        return GraphProfileState {
            filter_nodes_present,
            physical_output_present,
            target_output,
            ..GraphProfileState::default()
        };
    }

    if config.combined_output_enabled {
        let combined_sink = graph.nodes.iter().find(|node| {
            node.node_name.as_deref() == Some(PIPEWIRE_COMBINED_SINK_NODE)
                && node.media_class.as_deref() == Some("Audio/Sink")
                && node.node_virtual
        });
        let filter_to_combined = combined_sink
            .is_some_and(|combined| graph.has_valid_link(filter_stream.id, combined.id));
        let combined_to_physical =
            combined_sink
                .zip(physical_target)
                .is_some_and(|(combined, physical)| {
                    combined
                        .node_link_group
                        .as_deref()
                        .filter(|group| !group.is_empty())
                        .is_some_and(|combined_group| {
                            graph.nodes.iter().any(|node| {
                                node.media_class.as_deref() == Some("Stream/Output/Audio")
                                    && node.node_link_group.as_deref() == Some(combined_group)
                                    && graph.has_valid_link(node.id, physical.id)
                            })
                        })
                });

        GraphProfileState {
            filter_nodes_present,
            physical_output_present,
            target_linked: filter_to_combined && combined_to_physical,
            filter_active: filter_to_combined,
            combined_output_active: combined_to_physical,
            target_output,
        }
    } else {
        let target_linked = physical_target
            .is_some_and(|physical| graph.has_valid_link(filter_stream.id, physical.id));
        GraphProfileState {
            filter_nodes_present,
            physical_output_present,
            target_linked,
            filter_active: target_linked,
            combined_output_active: false,
            target_output,
        }
    }
}

impl PipeWireGraph {
    fn has_valid_link(&self, output_node_id: u32, input_node_id: u32) -> bool {
        self.links.iter().any(|link| {
            link.output_node_id == output_node_id
                && link.input_node_id == input_node_id
                && link
                    .error
                    .as_deref()
                    .is_none_or(|error| error.trim().is_empty())
                && matches!(link.state.as_deref(), Some("active" | "paused"))
        })
    }
}

fn node_display_name(node: &PipeWireNode) -> Option<String> {
    node.node_description
        .as_deref()
        .or(node.node_nick.as_deref())
        .or(node.node_name.as_deref())
        .map(ToOwned::to_owned)
}

fn detect_target_sink_from_graph(graph: &PipeWireGraph) -> Option<PipeWireNode> {
    let default_sink_name = detect_default_output_node_name();
    detect_target_sink_from_nodes_with_default(&graph.nodes, default_sink_name.as_deref())
}

fn detect_target_sink_from_nodes_with_default(
    nodes: &[PipeWireNode],
    default_sink_name: Option<&str>,
) -> Option<PipeWireNode> {
    if let Some(default_sink_name) = default_sink_name {
        if let Some(default_speaker) = nodes.iter().find(|node| {
            node.node_name.as_deref() == Some(default_sink_name) && is_internal_speaker_output(node)
        }) {
            return Some(default_speaker.clone());
        }
    }

    nodes
        .iter()
        .find(|node| is_internal_speaker_output(node))
        .cloned()
}

fn is_internal_speaker_output(node: &PipeWireNode) -> bool {
    is_physical_output(node)
        && [
            node.device_profile_description.as_deref(),
            node.node_description.as_deref(),
            node.node_nick.as_deref(),
            node.node_name.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|value| value.to_ascii_lowercase().contains("speaker"))
}

fn is_physical_output(node: &PipeWireNode) -> bool {
    node.media_class.as_deref() == Some("Audio/Sink")
        && !node.node_virtual
        && !is_galaxybook_sound_managed_node(node)
        && node
            .node_name
            .as_deref()
            .is_some_and(|name| !name.is_empty())
        && (node.device_id.is_some()
            || node
                .object_path
                .as_deref()
                .is_some_and(|path| path.starts_with("alsa:"))
            || node
                .node_name
                .as_deref()
                .is_some_and(|name| name.starts_with("alsa_output.")))
}

fn is_galaxybook_sound_managed_node(node: &PipeWireNode) -> bool {
    matches!(
        node.node_name.as_deref(),
        Some(PIPEWIRE_FILTER_MAIN_NODE | PIPEWIRE_FILTER_STREAM_NODE | PIPEWIRE_COMBINED_SINK_NODE)
    )
}

fn detect_default_output_node_name() -> Option<String> {
    let inspect = command_text("wpctl", &["inspect", "@DEFAULT_AUDIO_SINK@"]).ok()?;
    parse_pwctl_property(&inspect, "node.name")
}

fn parse_pwctl_property(output: &str, key: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let trimmed = line.trim();
        let candidate = trimmed.strip_prefix('*').map(str::trim).unwrap_or(trimmed);
        let prefix = format!("{key} = \"");
        candidate
            .strip_prefix(&prefix)
            .and_then(|value| value.strip_suffix('"'))
            .map(ToOwned::to_owned)
    })
}

fn pipewire_graph_with_retry(attempts: usize, delay: Duration) -> Result<PipeWireGraph, String> {
    let attempts = attempts.max(1);
    let mut last_error = None;
    for attempt in 0..attempts {
        match pipewire_graph() {
            Ok(graph) => return Ok(graph),
            Err(error) => last_error = Some(error),
        }
        if attempt + 1 < attempts {
            thread::sleep(delay);
        }
    }
    Err(last_error.unwrap_or_else(|| "PipeWire graph query failed".to_string()))
}

fn pipewire_graph_and_target_with_retry(
    attempts: usize,
    delay: Duration,
) -> Result<(PipeWireGraph, PipeWireNode), TargetQueryError> {
    let attempts = attempts.max(1);
    let default_sink_name = detect_default_output_node_name();
    let mut queried_graph = false;

    for attempt in 0..attempts {
        if let Ok(graph) = pipewire_graph() {
            queried_graph = true;
            if let Some(target) = detect_target_sink_from_nodes_with_default(
                &graph.nodes,
                default_sink_name.as_deref(),
            ) {
                return Ok((graph, target));
            }
        }
        if attempt + 1 < attempts {
            thread::sleep(delay);
        }
    }

    Err(if queried_graph {
        TargetQueryError::PhysicalOutputUnavailable
    } else {
        TargetQueryError::SessionUnavailable
    })
}

fn pipewire_graph() -> Result<PipeWireGraph, String> {
    let output = Command::new("pw-dump")
        .output()
        .map_err(|error| format!("failed to start pw-dump: {error}"))?;
    if !output.status.success() {
        return Err("pw-dump did not return a usable graph".to_string());
    }

    let objects = serde_json::from_slice::<Vec<PipeWireDumpObject>>(&output.stdout)
        .map_err(|error| format!("pw-dump returned invalid JSON: {error}"))?;
    pipewire_graph_from_objects(objects)
}

fn pipewire_graph_from_objects(objects: Vec<PipeWireDumpObject>) -> Result<PipeWireGraph, String> {
    let mut graph = PipeWireGraph::default();

    for object in objects {
        let Some(info) = object.info else {
            continue;
        };
        match object.object_type.as_str() {
            "PipeWire:Interface:Node" => graph.nodes.push(PipeWireNode {
                id: object.id,
                node_name: property_string(&info.props, "node.name"),
                node_description: property_string(&info.props, "node.description"),
                node_nick: property_string(&info.props, "node.nick"),
                media_class: property_string(&info.props, "media.class"),
                device_profile_description: property_string(
                    &info.props,
                    "device.profile.description",
                ),
                node_link_group: property_string(&info.props, "node.link-group"),
                object_path: property_string(&info.props, "object.path"),
                device_id: property_u32(&info.props, "device.id"),
                node_virtual: property_bool(&info.props, "node.virtual").unwrap_or(false),
            }),
            "PipeWire:Interface:Link" => {
                if let (Some(output_node_id), Some(input_node_id)) =
                    (info.output_node_id, info.input_node_id)
                {
                    graph.links.push(PipeWireLink {
                        output_node_id,
                        input_node_id,
                        state: info.state,
                        error: info.error,
                    });
                }
            }
            _ => {}
        }
    }

    if graph.nodes.is_empty() {
        Err("pw-dump returned no PipeWire nodes".to_string())
    } else {
        Ok(graph)
    }
}

fn property_string(props: &HashMap<String, Value>, key: &str) -> Option<String> {
    props
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn property_u32(props: &HashMap<String, Value>, key: &str) -> Option<u32> {
    props.get(key).and_then(|value| {
        value
            .as_u64()
            .and_then(|number| u32::try_from(number).ok())
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
    })
}

fn property_bool(props: &HashMap<String, Value>, key: &str) -> Option<bool> {
    props.get(key).and_then(|value| {
        value.as_bool().or_else(|| match value.as_str() {
            Some("true") => Some(true),
            Some("false") => Some(false),
            _ => None,
        })
    })
}

fn sound_app_config_path() -> PathBuf {
    config_home().join("galaxybook-sound").join("config.json")
}

fn pipewire_filter_config_path() -> PathBuf {
    config_home()
        .join("pipewire")
        .join("pipewire.conf.d")
        .join(PIPEWIRE_FILTER_CONFIG_NAME)
}

fn clamp_gain(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(-12.0, 12.0)
    } else {
        0.0
    }
}

fn escape_spa_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn command_text(command: &str, args: &[&str]) -> Result<String, ()> {
    let output = Command::new(command).args(args).output().map_err(|_| ())?;
    if !output.status.success() {
        return Err(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(())
        } else {
            Ok(stderr)
        }
    } else {
        Ok(stdout)
    }
}

fn systemd_available() -> bool {
    if !Path::new("/run/systemd/system").exists() {
        return false;
    }

    Command::new("systemctl")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn config_home() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|path| path.join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node(id: u32, name: &str, media_class: &str, link_group: Option<&str>) -> PipeWireNode {
        PipeWireNode {
            id,
            node_name: Some(name.to_string()),
            media_class: Some(media_class.to_string()),
            node_link_group: link_group.map(ToOwned::to_owned),
            node_virtual: true,
            ..PipeWireNode::default()
        }
    }

    fn test_speaker_node(id: u32, name: &str) -> PipeWireNode {
        PipeWireNode {
            id,
            node_name: Some(name.to_string()),
            node_description: Some("Notebook Speaker".into()),
            node_nick: Some("Speaker".into()),
            media_class: Some("Audio/Sink".into()),
            device_profile_description: Some("Speaker".into()),
            object_path: Some("alsa:pcm:0:playback".into()),
            device_id: Some(1),
            node_virtual: false,
            ..PipeWireNode::default()
        }
    }

    fn valid_link(output_node_id: u32, input_node_id: u32) -> PipeWireLink {
        PipeWireLink {
            output_node_id,
            input_node_id,
            state: Some("paused".into()),
            error: None,
        }
    }

    fn direct_filter_graph() -> PipeWireGraph {
        PipeWireGraph {
            nodes: vec![
                test_node(
                    10,
                    PIPEWIRE_FILTER_MAIN_NODE,
                    "Audio/Sink",
                    Some("filter-1"),
                ),
                test_node(
                    11,
                    PIPEWIRE_FILTER_STREAM_NODE,
                    "Stream/Output/Audio",
                    Some("filter-1"),
                ),
                test_speaker_node(12, "alsa_output.test.Speaker__sink"),
            ],
            links: Vec::new(),
        }
    }

    #[test]
    fn legacy_atmos_profile_normalizes_to_cinema_and_toggle() {
        let mut config = SoundAppConfig {
            selected_profile: AudioProfile::AtmosCompatible,
            atmos_enabled: false,
            combined_output_enabled: false,
            bands_db: [
                f64::NAN,
                -20.0,
                -12.0,
                -1.0,
                0.0,
                1.0,
                12.0,
                20.0,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ],
        };

        config.normalize();

        assert_eq!(config.selected_profile, AudioProfile::Cinema);
        assert!(config.atmos_enabled);
        assert_eq!(
            config.bands_db,
            [0.0, -12.0, -12.0, -1.0, 0.0, 1.0, 12.0, 12.0, 0.0, 0.0]
        );
    }

    #[test]
    fn preset_name_includes_atmos_suffix_only_when_enabled() {
        let mut config = SoundAppConfig {
            selected_profile: AudioProfile::Music,
            atmos_enabled: false,
            combined_output_enabled: false,
            bands_db: default_bands_for_profile(AudioProfile::Music),
        };

        assert_eq!(config.preset_name(), "Galaxy Book Sound - Música");

        config.atmos_enabled = true;
        assert_eq!(
            config.preset_name(),
            "Galaxy Book Sound - Música + Atmos compatível"
        );
    }

    #[test]
    fn pipewire_config_without_atmos_keeps_eq_only() {
        let config = SoundAppConfig::default();
        let target = test_speaker_node(1, "alsa_output.test.Speaker__sink");

        let conf = build_pipewire_filter_config(&config, &target);

        assert!(conf.contains("label = param_eq"));
        assert!(!conf.contains("name = widen_l"));
        assert!(!conf.contains("name = widen_r"));
        assert!(conf.contains("\"eq:Out 1\""));
        assert!(conf.contains("\"eq:Out 2\""));
        assert!(conf.contains("filter.smart = true"));
        assert!(!conf.contains("libpipewire-module-combine-stream"));
    }

    #[test]
    fn pipewire_config_rejects_empty_target_name_before_writing() {
        let target = PipeWireNode {
            node_name: Some("   ".into()),
            ..PipeWireNode::default()
        };

        assert!(validated_pipewire_filter_config(&SoundAppConfig::default(), &target).is_err());
    }

    #[test]
    fn pipewire_config_with_combined_output_adds_combine_sink() {
        let config = SoundAppConfig {
            selected_profile: AudioProfile::Flat,
            atmos_enabled: false,
            combined_output_enabled: true,
            bands_db: default_bands_for_profile(AudioProfile::Flat),
        };
        let target = test_speaker_node(1, "alsa_output.test.Speaker__sink");

        let conf = build_pipewire_filter_config(&config, &target);

        assert!(conf.contains("libpipewire-module-combine-stream"));
        assert!(conf.contains("combine.mode = sink"));
        assert!(conf.contains("combine.latency-compensate = true"));
        assert!(conf.contains("node.name = \"galaxybook_sound_combined_sink\""));
        assert!(
            conf.contains(
                "filter.smart.target = { node.name = \"galaxybook_sound_combined_sink\" }"
            )
        );
        assert!(conf.contains("node.name = \"alsa_output.test.Speaker__sink\""));
        assert!(!conf.contains("node.name = \"!~^"));
        assert!(!conf.contains("alsa_output.external.HDMI__sink"));
    }

    #[test]
    fn pipewire_config_with_atmos_adds_widening_stage() {
        let config = SoundAppConfig {
            selected_profile: AudioProfile::Cinema,
            atmos_enabled: true,
            combined_output_enabled: false,
            bands_db: default_bands_for_profile(AudioProfile::Cinema),
        };
        let target = test_speaker_node(1, "alsa_output.test.Speaker__sink");

        let conf = build_pipewire_filter_config(&config, &target);

        assert!(conf.contains("name = widen_l"));
        assert!(conf.contains("name = widen_r"));
        assert!(conf.contains("\"widen_l:Out\""));
        assert!(conf.contains("\"widen_r:Out\""));
        assert!(conf.contains("\"Gain 1\" = 1.08"));
        assert!(conf.contains("\"Gain 2\" = -0.08"));
        assert!(conf.contains("{ output = \"eq:Out 1\" input = \"widen_l:In 1\" }"));
        assert!(!conf.contains("{{ output = \"eq:Out 1\" input = \"widen_l:In 1\" }}"));
    }

    #[test]
    fn atmos_overlay_is_applied_on_top_of_manual_curve() {
        let config = SoundAppConfig {
            selected_profile: AudioProfile::Flat,
            atmos_enabled: true,
            combined_output_enabled: false,
            bands_db: [0.0; 10],
        };

        let effective = effective_bands_for_config(&config);

        assert_eq!(effective[0], 0.8);
        assert_eq!(effective[1], 1.1);
        assert_eq!(effective[9], 0.6);
    }

    #[test]
    fn legacy_config_without_combined_output_defaults_to_disabled() {
        let config = serde_json::from_str::<SoundAppConfig>(
            r#"{
                "selected_profile": "music",
                "atmos_enabled": true,
                "bands_db": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            }"#,
        )
        .expect("legacy config should deserialize");

        assert!(!config.combined_output_enabled);
    }

    #[test]
    fn virtual_app_sink_is_not_a_physical_output() {
        let virtual_sink = PipeWireNode {
            id: 1,
            node_name: Some(PIPEWIRE_COMBINED_SINK_NODE.into()),
            node_description: Some(PIPEWIRE_COMBINED_SINK_DESCRIPTION.into()),
            media_class: Some("Audio/Sink".into()),
            node_virtual: true,
            ..PipeWireNode::default()
        };

        assert!(!is_physical_output(&virtual_sink));
        assert!(is_physical_output(&test_speaker_node(
            2,
            "alsa_output.test.Speaker__sink"
        )));
    }

    #[test]
    fn target_sink_detection_ignores_own_filter_as_default_sink() {
        let nodes = vec![
            PipeWireNode {
                node_name: Some(PIPEWIRE_FILTER_MAIN_NODE.into()),
                node_description: Some("Galaxy Book Sound".into()),
                media_class: Some("Audio/Sink".into()),
                node_virtual: true,
                ..PipeWireNode::default()
            },
            PipeWireNode {
                node_name: Some("alsa_output.hdmi".into()),
                node_description: Some("External HDMI".into()),
                media_class: Some("Audio/Sink".into()),
                device_id: Some(2),
                ..PipeWireNode::default()
            },
        ];

        assert!(
            detect_target_sink_from_nodes_with_default(&nodes, Some(PIPEWIRE_FILTER_MAIN_NODE))
                .is_none()
        );
    }

    #[test]
    fn target_sink_detection_refuses_external_default_without_internal_speaker() {
        let nodes = vec![PipeWireNode {
            node_name: Some("alsa_output.usb".into()),
            node_description: Some("USB Audio".into()),
            media_class: Some("Audio/Sink".into()),
            device_id: Some(2),
            ..PipeWireNode::default()
        }];

        let target = detect_target_sink_from_nodes_with_default(&nodes, Some("alsa_output.usb"));

        assert!(target.is_none());
    }

    #[test]
    fn direct_profile_requires_both_filter_nodes_and_physical_link() {
        let config = SoundAppConfig::default();
        let mut graph = direct_filter_graph();

        let disconnected = evaluate_profile_graph(&graph, &config, None);
        assert!(disconnected.filter_nodes_present);
        assert!(disconnected.physical_output_present);
        assert!(!disconnected.filter_active);
        assert!(!disconnected.target_linked);

        graph.links.push(valid_link(11, 12));
        let connected = evaluate_profile_graph(&graph, &config, None);
        assert!(connected.filter_active);
        assert!(connected.target_linked);
        assert_eq!(connected.target_output.as_deref(), Some("Notebook Speaker"));

        graph.nodes[0].node_link_group = Some("wrong-group".into());
        let mismatched_pair = evaluate_profile_graph(&graph, &config, None);
        assert!(!mismatched_pair.filter_active);

        graph.nodes[0].node_link_group = None;
        graph.nodes[1].node_link_group = None;
        let unproven_pair = evaluate_profile_graph(&graph, &config, None);
        assert!(!unproven_pair.filter_active);
    }

    #[test]
    fn invalid_or_failed_link_does_not_activate_profile() {
        let config = SoundAppConfig::default();
        let mut graph = direct_filter_graph();
        graph.links.push(PipeWireLink {
            output_node_id: 11,
            input_node_id: 12,
            state: Some("init".into()),
            error: None,
        });
        assert!(!evaluate_profile_graph(&graph, &config, None).filter_active);

        graph.links[0].state = Some("active".into());
        graph.links[0].error = Some("link failed".into());
        assert!(!evaluate_profile_graph(&graph, &config, None).filter_active);
    }

    #[test]
    fn combined_profile_requires_filter_and_physical_links() {
        let config = SoundAppConfig {
            combined_output_enabled: true,
            ..SoundAppConfig::default()
        };
        let mut graph = direct_filter_graph();
        graph.nodes.push(PipeWireNode {
            id: 20,
            node_name: Some(PIPEWIRE_COMBINED_SINK_NODE.into()),
            media_class: Some("Audio/Sink".into()),
            node_link_group: Some("combine-1".into()),
            node_virtual: true,
            ..PipeWireNode::default()
        });
        graph.nodes.push(test_node(
            21,
            "output.combined.speaker",
            "Stream/Output/Audio",
            Some("combine-1"),
        ));
        graph.nodes.push(PipeWireNode {
            id: 13,
            node_name: Some("alsa_output.external.HDMI__sink".into()),
            node_description: Some("External HDMI".into()),
            media_class: Some("Audio/Sink".into()),
            device_id: Some(2),
            ..PipeWireNode::default()
        });
        graph.links.push(valid_link(11, 20));
        graph.links.push(valid_link(21, 13));

        let external_link_only = evaluate_profile_graph(&graph, &config, None);
        assert!(external_link_only.filter_active);
        assert!(!external_link_only.combined_output_active);
        assert!(!external_link_only.target_linked);

        graph.links.push(valid_link(21, 12));
        let connected = evaluate_profile_graph(&graph, &config, None);
        assert!(connected.filter_active);
        assert!(connected.combined_output_active);
        assert!(connected.target_linked);
    }

    #[test]
    fn parses_structured_pw_dump_nodes_and_links() {
        let objects = serde_json::from_str::<Vec<PipeWireDumpObject>>(
            r#"[
                {
                    "id": 40,
                    "type": "PipeWire:Interface:Node",
                    "info": {"props": {
                        "node.name": "galaxybook_sound_output",
                        "media.class": "Stream/Output/Audio",
                        "node.link-group": "filter-1",
                        "node.virtual": true
                    }}
                },
                {
                    "id": 68,
                    "type": "PipeWire:Interface:Node",
                    "info": {"props": {
                        "node.name": "alsa_output.test.Speaker__sink",
                        "node.description": "Notebook Speaker",
                        "media.class": "Audio/Sink",
                        "device.id": 7,
                        "node.virtual": false
                    }}
                },
                {
                    "id": 100,
                    "type": "PipeWire:Interface:Link",
                    "info": {
                        "output-node-id": 40,
                        "input-node-id": 68,
                        "state": "paused",
                        "error": null
                    }
                }
            ]"#,
        )
        .expect("fixture should deserialize");

        let graph = pipewire_graph_from_objects(objects).expect("graph should parse");

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.links.len(), 1);
        assert!(graph.nodes[0].node_virtual);
        assert_eq!(graph.nodes[1].device_id, Some(7));
        assert!(graph.has_valid_link(40, 68));
    }

    #[test]
    fn atomic_write_replaces_and_restores_configuration() {
        let unique = format!(
            "galaxybook-sound-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(unique);
        let path = directory.join("config.json");

        atomic_write(&path, b"old").expect("initial atomic write should succeed");
        let previous = read_optional_file(&path).expect("snapshot should succeed");
        atomic_write(&path, b"new").expect("replacement should succeed");
        assert_eq!(fs::read(&path).expect("new config should exist"), b"new");

        restore_optional_file(&path, previous.as_deref()).expect("restore should succeed");
        assert_eq!(fs::read(&path).expect("old config should exist"), b"old");
        assert!(
            fs::read_dir(&directory)
                .expect("test directory should exist")
                .all(|entry| !entry
                    .expect("directory entry should be readable")
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp"))
        );

        restore_optional_file(&path, None).expect("removal rollback should succeed");
        assert!(!path.exists());
        fs::remove_dir_all(&directory).expect("test directory should be removable");
    }
}
