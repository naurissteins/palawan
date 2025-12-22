package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/progress"
	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/huh"
)

var ansiRegexp = regexp.MustCompile("\x1b\\[[0-9;]*m")

type colors struct {
	green string
	red   string
	reset string
}

type installChoices struct {
	amdDrivers bool
	yay        bool
	hyprland   bool
	nodejs     bool
	fonts      bool
	codex      bool
	gemini     bool
	claude     bool
}

type step struct {
	label string
	run   func() error
}

type stepState struct {
	label  string
	status string
}

type stepStartMsg struct {
	index int
}

type stepDoneMsg struct {
	index int
}

type stepFailedMsg struct {
	index int
	err   error
}

type allDoneMsg struct{}

type progressModel struct {
	steps    []stepState
	current  int
	total    int
	spinner  spinner.Model
	progress progress.Model
	done     bool
	err      error
}

func newProgressModel(labels []string) progressModel {
	spin := spinner.New()
	spin.Spinner = spinner.Dot
	prog := progress.New(progress.WithDefaultGradient())
	prog.Width = 40

	states := make([]stepState, 0, len(labels))
	for _, label := range labels {
		states = append(states, stepState{label: label, status: "pending"})
	}

	return progressModel{
		steps:    states,
		current:  0,
		total:    len(labels),
		spinner:  spin,
		progress: prog,
	}
}

func (m progressModel) Init() tea.Cmd {
	return m.spinner.Tick
}

func (m progressModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "ctrl+c", "q":
			m.done = true
			m.err = fmt.Errorf("installation interrupted")
			return m, tea.Quit
		}
	case spinner.TickMsg:
		var cmd tea.Cmd
		m.spinner, cmd = m.spinner.Update(msg)
		return m, cmd
	case stepStartMsg:
		m.current = msg.index
		for i := range m.steps {
			if i < msg.index {
				m.steps[i].status = "done"
			} else if i == msg.index {
				m.steps[i].status = "running"
			}
		}
		return m, nil
	case stepDoneMsg:
		if msg.index >= 0 && msg.index < len(m.steps) {
			m.steps[msg.index].status = "done"
		}
		percent := float64(msg.index+1) / float64(m.total)
		return m, m.progress.SetPercent(percent)
	case stepFailedMsg:
		if msg.index >= 0 && msg.index < len(m.steps) {
			m.steps[msg.index].status = "failed"
		}
		m.done = true
		m.err = msg.err
		return m, tea.Quit
	case allDoneMsg:
		m.done = true
		return m, tea.Quit
	}

	return m, nil
}

func (m progressModel) View() string {
	var builder strings.Builder
	builder.WriteString("Palawan Installer\n\n")
	builder.WriteString(m.progress.View())
	builder.WriteString("\n\n")

	if m.current < len(m.steps) {
		builder.WriteString(fmt.Sprintf("%s %s\n\n", m.spinner.View(), m.steps[m.current].label))
	}

	for _, step := range m.steps {
		status := "[ ]"
		switch step.status {
		case "running":
			status = "[~]"
		case "done":
			status = "[x]"
		case "failed":
			status = "[!]"
		}
		builder.WriteString(fmt.Sprintf("%s %s\n", status, step.label))
	}

	if m.done && m.err != nil {
		builder.WriteString("\nError: ")
		builder.WriteString(m.err.Error())
		builder.WriteString("\n")
	}

	return builder.String()
}

func initColors() colors {
	if isTerminal(os.Stdout) {
		return colors{
			green: "\033[32m",
			red:   "\033[31m",
			reset: "\033[0m",
		}
	}
	return colors{}
}

func isTerminal(file *os.File) bool {
	info, err := file.Stat()
	if err != nil {
		return false
	}
	return (info.Mode() & os.ModeCharDevice) != 0
}

func stripANSI(text string) string {
	return ansiRegexp.ReplaceAllString(text, "")
}

func logLine(logFile *os.File, message string) {
	timestamp := time.Now().Format("2006-01-02 15:04:05")
	fmt.Fprintf(logFile, "[%s] %s\n", timestamp, message)
}

func startInstallLog(logFile *os.File, palette colors, showOutput bool) time.Time {
	start := time.Now()
	logLine(logFile, fmt.Sprintf("=== Palawan Installation Started: %s ===", start.Format("2006-01-02 15:04:05")))
	if showOutput {
		fmt.Printf("%sStarting Palawan installation...%s\n", palette.green, palette.reset)
	}
	return start
}

func stopInstallLog(logFile *os.File, start time.Time, palette colors, showOutput bool) {
	end := time.Now()
	logLine(logFile, fmt.Sprintf("=== Palawan Installation Completed: %s ===", end.Format("2006-01-02 15:04:05")))
	duration := int(end.Sub(start).Seconds())
	fmt.Fprintln(logFile, "")
	fmt.Fprintln(logFile, "[Installation Time Summary]")
	fmt.Fprintf(logFile, "Palawan:     %dm %ds\n", duration/60, duration%60)
	fmt.Fprintln(logFile, "=================================")
	if showOutput {
		fmt.Printf("%sInstallation complete.%s\n", palette.green, palette.reset)
	}
}

func streamOutput(reader io.Reader, logFile *os.File, showOutput bool) error {
	buffer := bufio.NewReader(reader)
	for {
		line, err := buffer.ReadString('\n')
		if line != "" {
			if showOutput {
				fmt.Print(line)
			}
			_, _ = logFile.WriteString(stripANSI(line))
		}
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
	}
}

func runCommand(command []string, logFile *os.File, palette colors, description string, showOutput bool) error {
	logLine(logFile, fmt.Sprintf("Starting: %s", description))
	cmd := exec.Command(command[0], command[1:]...)
	cmd.Env = os.Environ()
	output, err := cmd.StdoutPipe()
	if err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (pipe error: %v)", description, err))
		if showOutput {
			fmt.Printf("%sFailed: %s (pipe error)%s\n", palette.red, description, palette.reset)
		}
		return err
	}
	cmd.Stderr = cmd.Stdout

	if err := cmd.Start(); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (start error: %v)", description, err))
		if showOutput {
			fmt.Printf("%sFailed: %s (start error)%s\n", palette.red, description, palette.reset)
		}
		return err
	}

	if err := streamOutput(output, logFile, showOutput); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (stream error: %v)", description, err))
		if showOutput {
			fmt.Printf("%sFailed: %s (stream error)%s\n", palette.red, description, palette.reset)
		}
		_ = cmd.Wait()
		return err
	}

	if err := cmd.Wait(); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (exit error: %v)", description, err))
		if showOutput {
			fmt.Printf("%sFailed: %s (exit error)%s\n", palette.red, description, palette.reset)
		}
		return err
	}

	logLine(logFile, fmt.Sprintf("Completed: %s", description))
	return nil
}

func runScript(scriptPath string, installDir string, logFile *os.File, palette colors, showOutput bool) error {
	logLine(logFile, fmt.Sprintf("Starting: %s", scriptPath))
	command := fmt.Sprintf("source '%s/helpers/all.sh'; source '%s'", installDir, scriptPath)
	cmd := exec.Command("bash", "-c", command)
	cmd.Env = os.Environ()
	output, err := cmd.StdoutPipe()
	if err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (pipe error: %v)", scriptPath, err))
		if showOutput {
			fmt.Printf("%sFailed: %s (pipe error)%s\n", palette.red, scriptPath, palette.reset)
		}
		return err
	}
	cmd.Stderr = cmd.Stdout

	if err := cmd.Start(); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (start error: %v)", scriptPath, err))
		if showOutput {
			fmt.Printf("%sFailed: %s (start error)%s\n", palette.red, scriptPath, palette.reset)
		}
		return err
	}

	if err := streamOutput(output, logFile, showOutput); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (stream error: %v)", scriptPath, err))
		if showOutput {
			fmt.Printf("%sFailed: %s (stream error)%s\n", palette.red, scriptPath, palette.reset)
		}
		_ = cmd.Wait()
		return err
	}

	if err := cmd.Wait(); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (exit error: %v)", scriptPath, err))
		if showOutput {
			fmt.Printf("%sFailed: %s (exit error)%s\n", palette.red, scriptPath, palette.reset)
		}
		return err
	}

	logLine(logFile, fmt.Sprintf("Completed: %s", scriptPath))
	return nil
}

func installBasePackages(installDir string, logFile *os.File, palette colors, showOutput bool) error {
	packagesPath := filepath.Join(installDir, "palawan-base.packages")
	content, err := os.ReadFile(packagesPath)
	if err != nil {
		logLine(logFile, fmt.Sprintf("Missing packages file: %s", packagesPath))
		return err
	}

	var packages []string
	for _, line := range strings.Split(string(content), "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			continue
		}
		packages = append(packages, trimmed)
	}

	if len(packages) == 0 {
		logLine(logFile, "No base packages found; skipping.")
		return nil
	}

	command := append([]string{"sudo", "pacman", "-S", "--noconfirm", "--needed"}, packages...)
	return runCommand(command, logFile, palette, "install base packages", showOutput)
}

func promptChoices() (installChoices, error) {
	choices := installChoices{
		amdDrivers: true,
		yay:        true,
		hyprland:   true,
		nodejs:     true,
		fonts:      true,
		codex:      true,
		gemini:     true,
		claude:     true,
	}

	if !isTerminal(os.Stdout) || !isTerminal(os.Stdin) {
		return choices, nil
	}

	form := huh.NewForm(
		huh.NewGroup(
			huh.NewConfirm().Title("Install AMD drivers?").Value(&choices.amdDrivers),
			huh.NewConfirm().Title("Install yay (AUR helper)?").Value(&choices.yay),
			huh.NewConfirm().Title("Install Hyprland and Wayland deps?").Value(&choices.hyprland),
			huh.NewConfirm().Title("Install Node.js (via nvm)?").Value(&choices.nodejs),
			huh.NewConfirm().Title("Install Fonts?").Value(&choices.fonts),
			huh.NewConfirm().Title("Install Codex CLI?").Value(&choices.codex),
			huh.NewConfirm().Title("Install Gemini CLI?").Value(&choices.gemini),
			huh.NewConfirm().Title("Install Claude CLI?").Value(&choices.claude),
		),
	)

	if err := form.Run(); err != nil {
		return choices, err
	}

	return choices, nil
}

func setChoiceEnv(choices installChoices) {
	os.Setenv("PALAWAN_INSTALL_AMD_DRIVERS", boolEnv(choices.amdDrivers))
	os.Setenv("PALAWAN_INSTALL_YAY", boolEnv(choices.yay))
	os.Setenv("PALAWAN_INSTALL_HYPRLAND", boolEnv(choices.hyprland))
	os.Setenv("PALAWAN_INSTALL_NODEJS", boolEnv(choices.nodejs))
	os.Setenv("PALAWAN_INSTALL_FONTS", boolEnv(choices.fonts))
	os.Setenv("PALAWAN_INSTALL_CODEX_CLI", boolEnv(choices.codex))
	os.Setenv("PALAWAN_INSTALL_GEMINI_CLI", boolEnv(choices.gemini))
	os.Setenv("PALAWAN_INSTALL_CLAUDE_CLI", boolEnv(choices.claude))
}

func boolEnv(value bool) string {
	if value {
		return "1"
	}
	return "0"
}

func buildSteps(
	installDir string,
	logFile *os.File,
	palette colors,
	showOutput bool,
	choices installChoices,
) []step {
	var steps []step

	steps = append(steps, step{
		label: "base packages",
		run: func() error {
			return installBasePackages(installDir, logFile, palette, showOutput)
		},
	})

	addScript := func(label, path string) {
		steps = append(steps, step{
			label: label,
			run: func() error {
				if _, err := os.Stat(path); err != nil {
					logLine(logFile, fmt.Sprintf("Skipped missing: %s", path))
					return nil
				}
				return runScript(path, installDir, logFile, palette, showOutput)
			},
		})
	}

	if choices.amdDrivers {
		addScript("drivers/amd-gpu-cpu.sh", filepath.Join(installDir, "packages/drivers/amd-gpu-cpu.sh"))
	} else {
		logLine(logFile, "Skipped by choice: AMD drivers")
	}

	addScript("packages/yay.sh", filepath.Join(installDir, "packages/yay.sh"))
	addScript("packages/hyprland.sh", filepath.Join(installDir, "packages/hyprland.sh"))
	addScript("packages/nodejs.sh", filepath.Join(installDir, "packages/nodejs.sh"))
	addScript("packages/terminal.sh", filepath.Join(installDir, "packages/terminal.sh"))
	addScript("packages/ai-cli/codex-cli.sh", filepath.Join(installDir, "packages/ai-cli/codex-cli.sh"))
	addScript("packages/ai-cli/gemini-cli.sh", filepath.Join(installDir, "packages/ai-cli/gemini-cli.sh"))
	addScript("packages/ai-cli/claude-cli.sh", filepath.Join(installDir, "packages/ai-cli/claude-cli.sh"))
	addScript("packages/fonts.sh", filepath.Join(installDir, "packages/fonts.sh"))
	addScript("packages/web-browser.sh", filepath.Join(installDir, "packages/web-browser.sh"))
	addScript("packages/editors.sh", filepath.Join(installDir, "packages/editors.sh"))
	addScript("greeter/sddm/sddm.sh", filepath.Join(installDir, "greeter/sddm/sddm.sh"))
	addScript("post-install/gnome-theme.sh", filepath.Join(installDir, "post-install/gnome-theme.sh"))

	return steps
}

func filterOptionalSteps(steps []step, choices installChoices, logFile *os.File) []step {
	filtered := make([]step, 0, len(steps))
	for _, step := range steps {
		label := step.label
		switch label {
		case "packages/yay.sh":
			if !choices.yay {
				logLine(logFile, "Skipped by choice: yay")
				continue
			}
		case "packages/hyprland.sh":
			if !choices.hyprland {
				logLine(logFile, "Skipped by choice: hyprland")
				continue
			}
		case "packages/nodejs.sh":
			if !choices.nodejs {
				logLine(logFile, "Skipped by choice: nodejs")
				continue
			}
		case "packages/fonts.sh":
			if !choices.fonts {
				logLine(logFile, "Skipped by choice: fonts")
				continue
			}
		case "packages/ai-cli/codex-cli.sh":
			if !choices.codex {
				logLine(logFile, "Skipped by choice: codex cli")
				continue
			}
		case "packages/ai-cli/gemini-cli.sh":
			if !choices.gemini {
				logLine(logFile, "Skipped by choice: gemini cli")
				continue
			}
		case "packages/ai-cli/claude-cli.sh":
			if !choices.claude {
				logLine(logFile, "Skipped by choice: claude cli")
				continue
			}
		}
		filtered = append(filtered, step)
	}
	return filtered
}

func runStepsSequential(steps []step, logFile *os.File, palette colors, showOutput bool) error {
	for idx, step := range steps {
		if showOutput {
			barWidth := 30
			filled := (barWidth * (idx + 1)) / len(steps)
			bar := strings.Repeat("#", filled) + strings.Repeat("-", barWidth-filled)
			fmt.Printf("[%s] %d/%d %s\n", bar, idx+1, len(steps), step.label)
		}
		if err := step.run(); err != nil {
			return err
		}
	}
	return nil
}

func executeSteps(program *tea.Program, steps []step) {
	for idx, step := range steps {
		program.Send(stepStartMsg{index: idx})
		if err := step.run(); err != nil {
			program.Send(stepFailedMsg{index: idx, err: err})
			return
		}
		program.Send(stepDoneMsg{index: idx})
	}
	program.Send(allDoneMsg{})
}

func main() {
	palawanPath := os.Getenv("PALAWAN_PATH")
	if palawanPath == "" {
		homeDir, err := os.UserHomeDir()
		if err != nil {
			fmt.Println("Unable to determine home directory.")
			os.Exit(1)
		}
		palawanPath = filepath.Join(homeDir, ".local/share/palawan")
	}
	installDir := os.Getenv("PALAWAN_INSTALL")
	if installDir == "" {
		installDir = filepath.Join(palawanPath, "install")
	}
	logFilePath := os.Getenv("PALAWAN_INSTALL_LOG_FILE")
	if logFilePath == "" {
		logFilePath = filepath.Join(palawanPath, "install.log")
	}

	if err := os.MkdirAll(filepath.Dir(logFilePath), 0o755); err != nil {
		fmt.Printf("Failed to create log directory: %v\n", err)
		os.Exit(1)
	}

	logFile, err := os.Create(logFilePath)
	if err != nil {
		fmt.Printf("Failed to create log file: %v\n", err)
		os.Exit(1)
	}
	defer logFile.Close()

	currentPath := os.Getenv("PATH")
	_ = os.Setenv("PATH", fmt.Sprintf("%s/bin:%s", palawanPath, currentPath))

	choices, err := promptChoices()
	if err != nil {
		fmt.Printf("Failed to collect install choices: %v\n", err)
		os.Exit(1)
	}

	setChoiceEnv(choices)

	interactive := isTerminal(os.Stdout) && isTerminal(os.Stdin)
	showOutput := !interactive
	palette := initColors()
	start := startInstallLog(logFile, palette, showOutput)

	steps := buildSteps(installDir, logFile, palette, showOutput, choices)
	steps = filterOptionalSteps(steps, choices, logFile)

	if interactive {
		labels := make([]string, 0, len(steps))
		for _, step := range steps {
			labels = append(labels, step.label)
		}
		model := newProgressModel(labels)
		program := tea.NewProgram(model, tea.WithAltScreen())
		go executeSteps(program, steps)
		if _, err := program.Run(); err != nil {
			stopInstallLog(logFile, start, palette, showOutput)
			fmt.Printf("Installer error: %v\n", err)
			os.Exit(1)
		}
	} else {
		if err := runStepsSequential(steps, logFile, palette, showOutput); err != nil {
			stopInstallLog(logFile, start, palette, showOutput)
			os.Exit(1)
		}
	}

	stopInstallLog(logFile, start, palette, showOutput)
}
