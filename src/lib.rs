mod i18n;
mod updates;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

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
const PIPEWIRE_FILTER_CONFIG_NAME: &str = "99-galaxybook-sound.conf";
const RESTART_AUDIO_STACK_COMMAND: &str =
    "systemctl --user restart wireplumber.service pipewire.service pipewire-pulse.service";
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
        const PROFILES: [AudioProfile; 3] =
            [AudioProfile::Flat, AudioProfile::Music, AudioProfile::Cinema];
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
    pub bands_db: [f64; 10],
}

impl Default for SoundAppConfig {
    fn default() -> Self {
        let selected_profile = AudioProfile::default();
        Self {
            selected_profile,
            atmos_enabled: false,
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
    pub filter_active: bool,
    pub active_profile_name: Option<String>,
    pub target_output: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PipeWireNode {
    node_name: Option<String>,
    node_description: Option<String>,
    media_class: Option<String>,
    device_profile_description: Option<String>,
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            trf(
                "Falha ao preparar o diretório de configuração: {error}",
                &[("error", error.to_string())],
            )
        })?;
    }

    let json = serde_json::to_string_pretty(config).map_err(|error| {
        trf(
            "Falha ao serializar a configuração de áudio: {error}",
            &[("error", error.to_string())],
        )
    })?;
    fs::write(path, json).map_err(|error| {
        trf(
            "Falha ao salvar a configuração de áudio: {error}",
            &[("error", error.to_string())],
        )
    })
}

pub fn save_and_apply_profile(config: &SoundAppConfig) -> Result<String, String> {
    save_sound_app_config(config)?;

    let target = detect_target_sink().ok_or_else(|| {
        tr("Não foi possível identificar a saída interna do notebook para aplicar o pipeline.")
    })?;

    write_pipewire_filter_config(config, &target)?;
    restart_audio_stack()?;
    wait_for_filter_activation()?;

    Ok(config.preset_name())
}

pub fn collect_sound_session_state() -> SoundSessionState {
    let pipewire_available = command_exists("wpctl") && command_exists("pw-cli");
    let config_present = pipewire_filter_config_path().exists();

    if !pipewire_available {
        return SoundSessionState {
            pipewire_available: false,
            config_present,
            filter_active: false,
            active_profile_name: None,
            target_output: None,
        };
    }

    let nodes = pipewire_nodes();
    let filter_active = nodes.iter().any(|node| {
        node.node_name.as_deref() == Some(PIPEWIRE_FILTER_MAIN_NODE)
            && node.media_class.as_deref() == Some("Audio/Sink")
    });

    let target_output = detect_target_sink_from_nodes(&nodes)
        .and_then(|node| node.node_description.or(node.node_name))
        .or_else(detect_default_output_description);

    let active_profile_name = if filter_active {
        Some(load_sound_app_config().preset_name())
    } else {
        None
    };

    SoundSessionState {
        pipewire_available,
        config_present,
        filter_active,
        active_profile_name,
        target_output,
    }
}

pub fn run_smoke_test() -> Result<(), String> {
    let config = load_sound_app_config();
    let target = PipeWireNode {
        node_name: Some(
            "alsa_output.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__Speaker__sink".into(),
        ),
        node_description: Some("Meteor Lake-P HD Audio Controller Speaker".into()),
        media_class: Some("Audio/Sink".into()),
        device_profile_description: Some("Speaker".into()),
    };
    let conf_text = build_pipewire_filter_config(&config, &target);
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

fn write_pipewire_filter_config(config: &SoundAppConfig, target: &PipeWireNode) -> Result<(), String> {
    let target_name = target.node_name.as_deref().ok_or_else(|| {
        tr("A saída alvo do notebook não expôs um node.name válido no PipeWire.")
    })?;
    let path = pipewire_filter_config_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            trf(
                "Falha ao preparar o diretório do PipeWire: {error}",
                &[("error", error.to_string())],
            )
        })?;
    }

    let content = build_pipewire_filter_config(config, &PipeWireNode {
        node_name: Some(target_name.to_string()),
        node_description: target.node_description.clone(),
        media_class: target.media_class.clone(),
        device_profile_description: target.device_profile_description.clone(),
    });

    fs::write(&path, content).map_err(|error| {
        trf(
            "Falha ao gravar a configuração do PipeWire: {error}",
            &[("error", error.to_string())],
        )
    })
}

fn build_pipewire_filter_config(config: &SoundAppConfig, target: &PipeWireNode) -> String {
    let target_name = escape_spa_string(target.node_name.as_deref().unwrap_or_default());
    let target_label = escape_spa_string(
        target
            .node_description
            .as_deref()
            .or(target.node_name.as_deref())
            .unwrap_or_default(),
    );
    let graph = build_filter_graph(config);

    format!(
        "# Generated by {APP_NAME}. Changes will be overwritten.\n\
         # Target output: {target_label}\n\
         context.modules = [\n\
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

fn build_filter_graph(config: &SoundAppConfig) -> String {
    let filters = effective_bands_for_config(config);
    let filter_lines = EQUALIZER_FREQUENCIES_HZ
        .iter()
        .zip(filters.iter())
        .map(|(frequency_hz, gain_db)| {
            format!(
                "\t\t\t\t\t{{ type = bq_peaking freq = {:.1} gain = {:.2} q = 1.0 }}",
                *frequency_hz as f64,
                gain_db
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
        (String::new(), String::new(), "\t\t\t\t\"eq:Out 1\"\n\t\t\t\t\"eq:Out 2\"".to_string())
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
    for _ in 0..30 {
        if command_text("wpctl", &["status"]).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(150));
    }

    Err(tr(
        "O PipeWire não voltou a responder após o reinício da sessão de áudio.",
    ))
}

fn wait_for_filter_activation() -> Result<(), String> {
    for _ in 0..30 {
        if collect_sound_session_state().filter_active {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(150));
    }

    Err(tr(
        "O pipeline do Galaxy Book Sound não apareceu na sessão do PipeWire após aplicar a configuração.",
    ))
}

fn detect_target_sink() -> Option<PipeWireNode> {
    let nodes = pipewire_nodes();
    detect_target_sink_from_nodes(&nodes)
}

fn detect_target_sink_from_nodes(nodes: &[PipeWireNode]) -> Option<PipeWireNode> {
    let speaker = nodes.iter().find(|node| {
        node.media_class.as_deref() == Some("Audio/Sink")
            && node.node_name.as_deref() != Some(PIPEWIRE_FILTER_MAIN_NODE)
            && node.node_name.as_deref() != Some(PIPEWIRE_FILTER_STREAM_NODE)
            && (node.device_profile_description.as_deref() == Some("Speaker")
                || node
                    .node_description
                    .as_deref()
                    .is_some_and(|value| value.contains("Speaker"))
                || node
                    .node_name
                    .as_deref()
                    .is_some_and(|value| value.contains("Speaker__sink")))
    });

    if let Some(node) = speaker {
        return Some(node.clone());
    }

    let default_sink_name = detect_default_output_node_name()?;
    nodes.iter()
        .find(|node| node.node_name.as_deref() == Some(default_sink_name.as_str()))
        .cloned()
}

fn detect_default_output_node_name() -> Option<String> {
    let inspect = command_text("wpctl", &["inspect", "@DEFAULT_AUDIO_SINK@"]).ok()?;
    parse_pwctl_property(&inspect, "node.name")
}

fn detect_default_output_description() -> Option<String> {
    let inspect = command_text("wpctl", &["inspect", "@DEFAULT_AUDIO_SINK@"]).ok()?;
    parse_pwctl_property(&inspect, "node.description")
        .or_else(|| parse_pwctl_property(&inspect, "device.description"))
}

fn parse_pwctl_property(output: &str, key: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let trimmed = line.trim();
        let candidate = trimmed
            .strip_prefix('*')
            .map(str::trim)
            .unwrap_or(trimmed);
        let prefix = format!("{key} = \"");
        candidate
            .strip_prefix(&prefix)
            .and_then(|value| value.strip_suffix('"'))
            .map(ToOwned::to_owned)
    })
}

fn pipewire_nodes() -> Vec<PipeWireNode> {
    let Ok(output) = command_text("pw-cli", &["ls", "Node"]) else {
        return Vec::new();
    };

    let mut nodes = Vec::new();
    let mut current: Option<PipeWireNode> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("id ") && trimmed.contains("PipeWire:Interface:Node") {
            if let Some(node) = current.take() {
                nodes.push(node);
            }
            current = Some(PipeWireNode::default());
            continue;
        }

        let Some(node) = current.as_mut() else {
            continue;
        };

        if let Some(value) = parse_pw_cli_property(trimmed, "node.name") {
            node.node_name = Some(value);
        } else if let Some(value) = parse_pw_cli_property(trimmed, "node.description") {
            node.node_description = Some(value);
        } else if let Some(value) = parse_pw_cli_property(trimmed, "media.class") {
            node.media_class = Some(value);
        } else if let Some(value) = parse_pw_cli_property(trimmed, "device.profile.description") {
            node.device_profile_description = Some(value);
        }
    }

    if let Some(node) = current {
        nodes.push(node);
    }

    nodes
}

fn parse_pw_cli_property(line: &str, key: &str) -> Option<String> {
    let candidate = line.strip_prefix('*').map(str::trim).unwrap_or(line);
    let prefix = format!("{key} = ");
    let value = candidate.strip_prefix(&prefix)?;
    Some(value.trim_matches('"').to_string())
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
    value.clamp(-12.0, 12.0)
}

fn escape_spa_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn command_exists(command: &str) -> bool {
    Command::new("bash")
        .args(["-lc", &format!("command -v {command} >/dev/null 2>&1")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
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

    #[test]
    fn legacy_atmos_profile_normalizes_to_cinema_and_toggle() {
        let mut config = SoundAppConfig {
            selected_profile: AudioProfile::AtmosCompatible,
            atmos_enabled: false,
            bands_db: [0.0; 10],
        };

        config.normalize();

        assert_eq!(config.selected_profile, AudioProfile::Cinema);
        assert!(config.atmos_enabled);
    }

    #[test]
    fn preset_name_includes_atmos_suffix_only_when_enabled() {
        let mut config = SoundAppConfig {
            selected_profile: AudioProfile::Music,
            atmos_enabled: false,
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
        let target = PipeWireNode {
            node_name: Some("alsa_output.test.Speaker__sink".into()),
            node_description: Some("Notebook Speaker".into()),
            media_class: Some("Audio/Sink".into()),
            device_profile_description: Some("Speaker".into()),
        };

        let conf = build_pipewire_filter_config(&config, &target);

        assert!(conf.contains("label = param_eq"));
        assert!(!conf.contains("name = widen_l"));
        assert!(!conf.contains("name = widen_r"));
        assert!(conf.contains("\"eq:Out 1\""));
        assert!(conf.contains("\"eq:Out 2\""));
        assert!(conf.contains("filter.smart = true"));
    }

    #[test]
    fn pipewire_config_with_atmos_adds_widening_stage() {
        let config = SoundAppConfig {
            selected_profile: AudioProfile::Cinema,
            atmos_enabled: true,
            bands_db: default_bands_for_profile(AudioProfile::Cinema),
        };
        let target = PipeWireNode {
            node_name: Some("alsa_output.test.Speaker__sink".into()),
            node_description: Some("Notebook Speaker".into()),
            media_class: Some("Audio/Sink".into()),
            device_profile_description: Some("Speaker".into()),
        };

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
            bands_db: [0.0; 10],
        };

        let effective = effective_bands_for_config(&config);

        assert_eq!(effective[0], 0.8);
        assert_eq!(effective[1], 1.1);
        assert_eq!(effective[9], 0.6);
    }
}
