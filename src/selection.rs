#[derive(Default, Clone)]
pub struct PackageSelection {
    pub pacman: Vec<String>,
    pub yay: Vec<String>,
}

impl PackageSelection {
    pub fn is_empty(&self) -> bool {
        self.pacman.is_empty() && self.yay.is_empty()
    }
}

#[derive(Default, Clone)]
pub struct NpmSelection {
    pub packages: Vec<String>,
}

impl NpmSelection {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

pub struct InstallChoice {
    pub label: &'static str,
    pub pacman: &'static [&'static str],
    pub yay: &'static [&'static str],
}

pub struct NpmChoice {
    pub label: &'static str,
    pub packages: &'static [&'static str],
}

const FIREFOX_PACMAN: [&str; 2] = ["firefox", "firefox-ublock-origin"];
const CHROMIUM_PACMAN: [&str; 1] = ["chromium"];
const UNGOOGLED_YAY: [&str; 1] = ["ungoogled-chromium-bin"];
const BRAVE_YAY: [&str; 1] = ["brave-bin"];
const ZEN_YAY: [&str; 1] = ["zen-browser-bin"];
const LIBREWOLF_YAY: [&str; 1] = ["librewolf-bin"];
const GHOSTTY_PACMAN: [&str; 1] = ["ghostty"];
const KITTY_PACMAN: [&str; 1] = ["kitty"];
const ALACRITTY_PACMAN: [&str; 1] = ["alacritty"];
const ZED_PACMAN: [&str; 1] = ["zed"];
const CURSOR_YAY: [&str; 1] = ["cursor-bin"];
const VSCODE_YAY: [&str; 1] = ["visual-studio-code-bin"];
const VSCODIUM_YAY: [&str; 1] = ["vscodium-bin"];
const SUBLIME_YAY: [&str; 1] = ["sublime-text-4"];
const CODEX_NPM: [&str; 1] = ["@openai/codex"];
const GEMINI_NPM: [&str; 1] = ["@google/gemini-cli"];
const CLAUDE_NPM: [&str; 1] = ["@anthropic-ai/claude-code"];

pub const BROWSER_CHOICES: [InstallChoice; 6] = [
    InstallChoice {
        label: "Firefox",
        pacman: &FIREFOX_PACMAN,
        yay: &[],
    },
    InstallChoice {
        label: "Chromium",
        pacman: &CHROMIUM_PACMAN,
        yay: &[],
    },
    InstallChoice {
        label: "Ungoogled Chromium",
        pacman: &[],
        yay: &UNGOOGLED_YAY,
    },
    InstallChoice {
        label: "Brave",
        pacman: &[],
        yay: &BRAVE_YAY,
    },
    InstallChoice {
        label: "Zen Browser",
        pacman: &[],
        yay: &ZEN_YAY,
    },
    InstallChoice {
        label: "LibreWolf",
        pacman: &[],
        yay: &LIBREWOLF_YAY,
    },
];

pub const TERMINAL_CHOICES: [InstallChoice; 3] = [
    InstallChoice {
        label: "Ghostty",
        pacman: &GHOSTTY_PACMAN,
        yay: &[],
    },
    InstallChoice {
        label: "Kitty",
        pacman: &KITTY_PACMAN,
        yay: &[],
    },
    InstallChoice {
        label: "Alacritty",
        pacman: &ALACRITTY_PACMAN,
        yay: &[],
    },
];

pub const EDITOR_CHOICES: [InstallChoice; 5] = [
    InstallChoice {
        label: "Zed",
        pacman: &ZED_PACMAN,
        yay: &[],
    },
    InstallChoice {
        label: "Cursor",
        pacman: &[],
        yay: &CURSOR_YAY,
    },
    InstallChoice {
        label: "Visual Studio Code",
        pacman: &[],
        yay: &VSCODE_YAY,
    },
    InstallChoice {
        label: "VSCodium",
        pacman: &[],
        yay: &VSCODIUM_YAY,
    },
    InstallChoice {
        label: "Sublime Text 4",
        pacman: &[],
        yay: &SUBLIME_YAY,
    },
];

pub const CODING_AGENT_CHOICES: [NpmChoice; 3] = [
    NpmChoice {
        label: "Codex",
        packages: &CODEX_NPM,
    },
    NpmChoice {
        label: "Gemini",
        packages: &GEMINI_NPM,
    },
    NpmChoice {
        label: "Claude",
        packages: &CLAUDE_NPM,
    },
];

pub fn selection_from_flags(flags: &[bool]) -> PackageSelection {
    selection_from_flags_for(flags, &BROWSER_CHOICES)
}

pub fn selection_from_flags_for(
    flags: &[bool],
    choices: &[InstallChoice],
) -> PackageSelection {
    let mut selection = PackageSelection::default();
    for (flag, choice) in flags.iter().copied().zip(choices.iter()) {
        if flag {
            extend_unique(&mut selection.pacman, choice.pacman);
            extend_unique(&mut selection.yay, choice.yay);
        }
    }
    selection
}

pub fn selection_from_flags_for_npm(
    flags: &[bool],
    choices: &[NpmChoice],
) -> NpmSelection {
    let mut selection = NpmSelection::default();
    for (flag, choice) in flags.iter().copied().zip(choices.iter()) {
        if flag {
            extend_unique(&mut selection.packages, choice.packages);
        }
    }
    selection
}

pub fn labels_for_selection(selection: &PackageSelection, choices: &[InstallChoice]) -> Vec<String> {
    let mut labels = Vec::new();
    for choice in choices {
        if choice_selected(selection, choice) {
            labels.push(choice.label.to_string());
        }
    }
    labels
}

pub fn labels_for_npm_selection(selection: &NpmSelection, choices: &[NpmChoice]) -> Vec<String> {
    let mut labels = Vec::new();
    for choice in choices {
        let mut matched = true;
        for pkg in choice.packages {
            if !selection.packages.iter().any(|installed| installed == pkg) {
                matched = false;
                break;
            }
        }
        if matched {
            labels.push(choice.label.to_string());
        }
    }
    labels
}

pub fn flags_for_selection(selection: &PackageSelection, choices: &[InstallChoice]) -> Vec<bool> {
    choices
        .iter()
        .map(|choice| choice_selected(selection, choice))
        .collect()
}

pub fn flags_for_npm_selection(selection: &NpmSelection, choices: &[NpmChoice]) -> Vec<bool> {
    choices
        .iter()
        .map(|choice| {
            choice
                .packages
                .iter()
                .all(|pkg| selection.packages.iter().any(|installed| installed == pkg))
        })
        .collect()
}

fn choice_selected(selection: &PackageSelection, choice: &InstallChoice) -> bool {
    for pkg in choice.pacman {
        if !selection.pacman.iter().any(|installed| installed == pkg) {
            return false;
        }
    }
    for pkg in choice.yay {
        if !selection.yay.iter().any(|installed| installed == pkg) {
            return false;
        }
    }
    !choice.pacman.is_empty() || !choice.yay.is_empty()
}

fn extend_unique(target: &mut Vec<String>, values: &[&str]) {
    for value in values {
        if !target.iter().any(|existing| existing == value) {
            target.push(value.to_string());
        }
    }
}
