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

pub struct InstallChoice {
    pub label: &'static str,
    pub pacman: &'static [&'static str],
    pub yay: &'static [&'static str],
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

fn extend_unique(target: &mut Vec<String>, values: &[&str]) {
    for value in values {
        if !target.iter().any(|existing| existing == value) {
            target.push(value.to_string());
        }
    }
}
