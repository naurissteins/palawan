package main

import (
	"bufio"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"
	"sync"
	"time"

	"github.com/charmbracelet/bubbles/progress"
	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

type stepStatus int

const (
	stepPending stepStatus = iota
	stepRunning
	stepDone
	stepFailed
)

type installStep struct {
	Name   string
	Status stepStatus
	Err    error
}

type logMsg string

type progressMsg float64

type stepMsg struct {
	Index  int
	Status stepStatus
	Err    error
}

type doneMsg struct {
	Err error
}

type model struct {
	spinner  spinner.Model
	progress progress.Model
	viewport viewport.Model
	steps    []installStep
	logs     []string
	events   chan tea.Msg
	width    int
	height   int
	done     bool
	err      error
}

func main() {
	if err := ensureSudo(); err != nil {
		fmt.Fprintln(os.Stderr, "sudo is required:", err)
		os.Exit(1)
	}

	p := tea.NewProgram(initialModel(), tea.WithAltScreen())
	if err := p.Start(); err != nil {
		fmt.Fprintln(os.Stderr, "failed to start installer:", err)
		os.Exit(1)
	}
}

func ensureSudo() error {
	if os.Geteuid() == 0 {
		return nil
	}
	cmd := exec.Command("sudo", "-v")
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func initialModel() model {
	s := spinner.New()
	s.Spinner = spinner.Line
	p := progress.New(progress.WithDefaultGradient(), progress.WithWidth(40))
	vp := viewport.New(0, 0)
	vp.MouseWheelEnabled = true

	steps := []installStep{
		{Name: "Installing base packages", Status: stepPending},
		{Name: "Finalizing", Status: stepPending},
	}

	return model{
		spinner:  s,
		progress: p,
		viewport: vp,
		steps:    steps,
		logs:     []string{"Starting Palawan installer..."},
		events:   make(chan tea.Msg, 128),
	}
}

func (m model) Init() tea.Cmd {
	go runInstall(m.events)
	return tea.Batch(m.spinner.Tick, waitForEvent(m.events))
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var (
		cmd  tea.Cmd
		cmds []tea.Cmd
	)

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c":
			return m, tea.Quit
		case "up", "k":
			m.viewport.LineUp(1)
		case "down", "j":
			m.viewport.LineDown(1)
		case "pgup":
			m.viewport.HalfViewUp()
		case "pgdown":
			m.viewport.HalfViewDown()
		}
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.viewport.Width = msg.Width - 4
		m.viewport.Height = logHeight(msg.Height)
		m.viewport.YPosition = 0
		m.refreshViewport()
	case spinner.TickMsg:
		m.spinner, cmd = m.spinner.Update(msg)
		cmds = append(cmds, cmd)
	case progress.FrameMsg:
		updated, cmd := m.progress.Update(msg)
		if progressModel, ok := updated.(progress.Model); ok {
			m.progress = progressModel
		}
		cmds = append(cmds, cmd)
	case logMsg:
		m.logs = append(m.logs, string(msg))
		m.refreshViewport()
		cmds = append(cmds, waitForEvent(m.events))
	case progressMsg:
		cmd = m.progress.SetPercent(float64(msg))
		cmds = append(cmds, cmd, waitForEvent(m.events))
	case stepMsg:
		if msg.Index >= 0 && msg.Index < len(m.steps) {
			m.steps[msg.Index].Status = msg.Status
			m.steps[msg.Index].Err = msg.Err
		}
		cmds = append(cmds, waitForEvent(m.events))
	case doneMsg:
		m.done = true
		m.err = msg.Err
	}

	cmds = append(cmds, m.spinner.Tick)
	return m, tea.Batch(cmds...)
}

func (m model) View() string {
	var b strings.Builder

	titleStyle := lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#F25F5C"))
	subtleStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("#5B5F66"))
	progressStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("#247BA0"))
	logStyle := lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).Padding(0, 1)

	b.WriteString(titleStyle.Render("Palawan Installer"))
	b.WriteString("\n")

	progressLine := fmt.Sprintf("Progress: %s %3.0f%%", m.progress.View(), m.progress.Percent()*100)
	b.WriteString(progressStyle.Render(progressLine))
	b.WriteString("\n\n")

	for _, step := range m.steps {
		b.WriteString(renderStep(step, m.spinner.View()))
		b.WriteString("\n")
	}

	if m.done {
		if m.err != nil {
			b.WriteString("\n" + lipgloss.NewStyle().Foreground(lipgloss.Color("#C81D25")).Render("Installation failed."))
		} else {
			b.WriteString("\n" + lipgloss.NewStyle().Foreground(lipgloss.Color("#2A9D8F")).Render("Installation complete."))
		}
		b.WriteString("\n" + subtleStyle.Render("Press q to quit."))
	}

	b.WriteString("\n\n")

	logTitle := subtleStyle.Render("Logs")
	b.WriteString(logTitle)
	b.WriteString("\n")
	b.WriteString(logStyle.Render(m.viewport.View()))

	return b.String()
}

func renderStep(step installStep, spinnerView string) string {
	var icon string
	style := lipgloss.NewStyle()

	switch step.Status {
	case stepPending:
		icon = "[ ]"
		style = style.Foreground(lipgloss.Color("#8D99AE"))
	case stepRunning:
		icon = "[..]"
		style = style.Foreground(lipgloss.Color("#F4A261"))
	case stepDone:
		icon = "[OK]"
		style = style.Foreground(lipgloss.Color("#2A9D8F"))
	case stepFailed:
		icon = "[x]"
		style = style.Foreground(lipgloss.Color("#C81D25"))
	}

	line := fmt.Sprintf("%s %s", icon, step.Name)
	if step.Status == stepRunning {
		line = fmt.Sprintf("%s %s", line, spinnerView)
	}

	if step.Err != nil {
		line = fmt.Sprintf("%s (%v)", line, step.Err)
	}

	return style.Render(line)
}

func logHeight(totalHeight int) int {
	height := totalHeight - 10
	if height < 6 {
		return 6
	}
	return height
}

func (m *model) refreshViewport() {
	content := strings.Join(m.logs, "\n")
	wasAtBottom := m.viewport.AtBottom()
	m.viewport.SetContent(content)
	if wasAtBottom {
		m.viewport.GotoBottom()
	}
}

func waitForEvent(events <-chan tea.Msg) tea.Cmd {
	return func() tea.Msg {
		msg, ok := <-events
		if !ok {
			return nil
		}
		return msg
	}
}

func runInstall(events chan<- tea.Msg) {
	defer close(events)

	packages := []string{"fastfetch", "htop", "vim", "neovim"}

	events <- stepMsg{Index: 0, Status: stepRunning}
	events <- logMsg("Installing base packages...")

	for idx, pkg := range packages {
		events <- logMsg(fmt.Sprintf("Installing %s...", pkg))
		if err := runCommand(events, "sudo", "pacman", "-S", "--noconfirm", "--needed", pkg); err != nil {
			events <- logMsg(fmt.Sprintf("Failed to install %s: %v", pkg, err))
			events <- stepMsg{Index: 0, Status: stepFailed, Err: err}
			events <- doneMsg{Err: err}
			return
		}
		percent := float64(idx+1) / float64(len(packages))
		events <- progressMsg(percent)
	}

	events <- stepMsg{Index: 0, Status: stepDone}
	events <- stepMsg{Index: 1, Status: stepRunning}
	events <- logMsg("Finalizing...")
	time.Sleep(500 * time.Millisecond)
	events <- stepMsg{Index: 1, Status: stepDone}
	events <- progressMsg(1.0)
	events <- doneMsg{}
}

func runCommand(events chan<- tea.Msg, name string, args ...string) error {
	cmd := exec.Command(name, args...)
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return err
	}
	stderr, err := cmd.StderrPipe()
	if err != nil {
		return err
	}

	if err := cmd.Start(); err != nil {
		return err
	}

	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		defer wg.Done()
		streamLogs(events, stdout)
	}()
	go func() {
		defer wg.Done()
		streamLogs(events, stderr)
	}()

	wg.Wait()
	if err := cmd.Wait(); err != nil {
		return err
	}
	return nil
}

func streamLogs(events chan<- tea.Msg, r io.Reader) {
	scanner := bufio.NewScanner(r)
	for scanner.Scan() {
		line := scanner.Text()
		if strings.TrimSpace(line) == "" {
			continue
		}
		events <- logMsg(line)
	}
	if err := scanner.Err(); err != nil && !errors.Is(err, io.EOF) {
		events <- logMsg(fmt.Sprintf("log stream error: %v", err))
	}
}
