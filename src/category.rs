use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessCategory {
    Browser,
    Development,
    System,
    Communication,
    Media,
    Gaming,
    App,
    Other,
}

impl ProcessCategory {
    pub fn label(&self) -> &'static str {
        match self {
            ProcessCategory::Browser => "🌐 Browser",
            ProcessCategory::Development => "💻 Development & Tools",
            ProcessCategory::System => "⚙️ Windows System",
            ProcessCategory::Communication => "💬 Communication",
            ProcessCategory::Media => "🎵 Media & Creative",
            ProcessCategory::Gaming => "🎮 Gaming",
            ProcessCategory::App => "📦 Applications",
            ProcessCategory::Other => "📁 Other Processes",
        }
    }

    pub fn base_hue(&self) -> f64 {
        match self {
            ProcessCategory::Browser => 210.0,      // Blue
            ProcessCategory::Development => 275.0,  // Purple
            ProcessCategory::System => 200.0,       // Slate / Cool Grey
            ProcessCategory::Communication => 160.0,// Teal / Cyan
            ProcessCategory::Media => 330.0,        // Pink / Magenta
            ProcessCategory::Gaming => 120.0,       // Green
            ProcessCategory::App => 30.0,           // Amber / Orange
            ProcessCategory::Other => 45.0,         // Muted Gold
        }
    }
}

pub fn classify_process(name: &str) -> ProcessCategory {
    let lower = name.to_lowercase();

    // Browsers
    if lower.starts_with("chrome")
        || lower.starts_with("msedge")
        || lower.starts_with("firefox")
        || lower.starts_with("brave")
        || lower.starts_with("vivaldi")
        || lower.starts_with("opera")
        || lower.starts_with("arc")
    {
        return ProcessCategory::Browser;
    }

    // Development & Terminals
    if lower.starts_with("code")
        || lower.starts_with("cursor")
        || lower.starts_with("devenv")
        || lower.starts_with("rust")
        || lower.starts_with("cargo")
        || lower.starts_with("node")
        || lower.starts_with("python")
        || lower.starts_with("git")
        || lower.starts_with("powershell")
        || lower.starts_with("pwsh")
        || lower.starts_with("cmd")
        || lower.starts_with("windowsterminal")
        || lower.starts_with("wt")
        || lower.starts_with("docker")
        || lower.starts_with("wsl")
        || lower.starts_with("idea")
        || lower.starts_with("clion")
    {
        return ProcessCategory::Development;
    }

    // Windows System & Core
    if lower.starts_with("svchost")
        || lower.starts_with("explorer")
        || lower.starts_with("system")
        || lower.starts_with("registry")
        || lower.starts_with("smss")
        || lower.starts_with("csrss")
        || lower.starts_with("wininit")
        || lower.starts_with("services")
        || lower.starts_with("lsass")
        || lower.starts_with("winlogon")
        || lower.starts_with("dwm")
        || lower.starts_with("fontdrvhost")
        || lower.starts_with("sihost")
        || lower.starts_with("taskhostw")
        || lower.starts_with("runtimebroker")
        || lower.starts_with("shellexperiencehost")
        || lower.starts_with("searchhost")
        || lower.starts_with("startmenuexperiencehost")
        || lower.starts_with("ctfmon")
        || lower.starts_with("securityhealthservice")
        || lower.starts_with("msmpeng")
        || lower.starts_with("audiodg")
        || lower.starts_with("spoolsv")
        || lower.starts_with("wmpnetwk")
        || lower.starts_with("conhost")
    {
        return ProcessCategory::System;
    }

    // Communication
    if lower.starts_with("slack")
        || lower.starts_with("teams")
        || lower.starts_with("discord")
        || lower.starts_with("zoom")
        || lower.starts_with("skype")
        || lower.starts_with("telegram")
        || lower.starts_with("line")
        || lower.starts_with("thunderbird")
        || lower.starts_with("outlook")
    {
        return ProcessCategory::Communication;
    }

    // Media & Creative
    if lower.starts_with("spotify")
        || lower.starts_with("vlc")
        || lower.starts_with("photoshop")
        || lower.starts_with("illustrator")
        || lower.starts_with("premiere")
        || lower.starts_with("obs64")
        || lower.starts_with("obs")
        || lower.starts_with("audacity")
        || lower.starts_with("foobar2000")
        || lower.starts_with("musicbee")
    {
        return ProcessCategory::Media;
    }

    // Gaming
    if lower.starts_with("steam")
        || lower.starts_with("epicgameslauncher")
        || lower.starts_with("battle.net")
        || lower.starts_with("riotclientservices")
        || lower.starts_with("origin")
        || lower.starts_with("ea")
        || lower.starts_with("gog")
    {
        return ProcessCategory::Gaming;
    }

    if lower.ends_with(".exe") {
        ProcessCategory::App
    } else {
        ProcessCategory::Other
    }
}

/// Group a list of items into category buckets or process-name buckets
#[derive(Debug, Clone)]
pub struct ProcessGroup<T> {
    pub key: String,
    pub title: String,
    pub category: ProcessCategory,
    pub total_working_set: u64,
    pub total_private: u64,
    pub total_gpu_dedicated: u64,
    pub total_gpu_shared: u64,
    pub items: Vec<T>,
}

pub fn group_by_name(entries: &[crate::memory::ProcessMemoryEntry]) -> Vec<ProcessGroup<crate::memory::ProcessMemoryEntry>> {
    let mut map: HashMap<String, Vec<crate::memory::ProcessMemoryEntry>> = HashMap::new();
    for entry in entries {
        map.entry(entry.name.clone()).or_default().push(entry.clone());
    }

    let mut groups = Vec::new();
    for (name, items) in map.into_iter() {
        let category = classify_process(&name);
        let total_working_set: u64 = items.iter().map(|p| p.working_set_bytes).sum();
        let total_private: u64 = items.iter().map(|p| p.private_bytes).sum();
        let total_gpu_dedicated: u64 = items.iter().map(|p| p.gpu_dedicated_bytes).sum();
        let total_gpu_shared: u64 = items.iter().map(|p| p.gpu_shared_bytes).sum();
        let count = items.len();
        let title = if count > 1 {
            format!("{} ({} processes)", name, count)
        } else {
            name.clone()
        };

        groups.push(ProcessGroup {
            key: name,
            title,
            category,
            total_working_set,
            total_private,
            total_gpu_dedicated,
            total_gpu_shared,
            items,
        });
    }

    groups
}

pub fn group_by_category(entries: &[crate::memory::ProcessMemoryEntry]) -> Vec<ProcessGroup<crate::memory::ProcessMemoryEntry>> {
    let mut map: HashMap<ProcessCategory, Vec<crate::memory::ProcessMemoryEntry>> = HashMap::new();
    for entry in entries {
        let cat = classify_process(&entry.name);
        map.entry(cat).or_default().push(entry.clone());
    }

    let mut groups = Vec::new();
    for (cat, items) in map.into_iter() {
        let total_working_set: u64 = items.iter().map(|p| p.working_set_bytes).sum();
        let total_private: u64 = items.iter().map(|p| p.private_bytes).sum();
        let total_gpu_dedicated: u64 = items.iter().map(|p| p.gpu_dedicated_bytes).sum();
        let total_gpu_shared: u64 = items.iter().map(|p| p.gpu_shared_bytes).sum();
        let count = items.len();
        let title = format!("{} ({} processes)", cat.label(), count);

        groups.push(ProcessGroup {
            key: format!("{:?}", cat),
            title,
            category: cat,
            total_working_set,
            total_private,
            total_gpu_dedicated,
            total_gpu_shared,
            items,
        });
    }

    groups
}
