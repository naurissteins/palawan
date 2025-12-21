# Function to ask a yes/no question
init_prompt_colors() {
    if [ -n "${PROMPT_COLORS_INIT:-}" ]; then
        return
    fi

    if [ -t 1 ] || [ -t 2 ] || [ -t 0 ] || [ -w /dev/tty ]; then
        PROMPT_COLOR=$'\033[33m'
        SECTION_COLOR=$'\033[36m'
        RESET_COLOR=$'\033[0m'
    else
        PROMPT_COLOR=""
        SECTION_COLOR=""
        RESET_COLOR=""
    fi

    PROMPT_COLORS_INIT=1
}

ask_yes_no() {
    while true; do
        init_prompt_colors
        read -p "${PROMPT_COLOR}$1 [y/n]: ${RESET_COLOR}" yn </dev/tty
        case $yn in
            [Yy]* ) return 0;; # Yes
            [Nn]* ) return 1;; # No
            * ) echo "Please answer yes or no.";;
        esac
    done
}

print_section() {
    local text="$1"
    local width border

    init_prompt_colors

    width=${#text}
    border=$(printf '%*s' $((width + 2)) '' | tr ' ' '-')

    printf '%b\n' "${SECTION_COLOR}+${border}+${RESET_COLOR}"
    printf '%b\n' "${SECTION_COLOR}| ${text} |${RESET_COLOR}"
    printf '%b\n' "${SECTION_COLOR}+${border}+${RESET_COLOR}"
}
