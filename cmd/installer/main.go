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
)

var ansiRegexp = regexp.MustCompile("\x1b\\[[0-9;]*m")

type colors struct {
	green string
	red   string
	reset string
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

func startInstallLog(logFile *os.File, palette colors) time.Time {
	start := time.Now()
	logLine(logFile, fmt.Sprintf("=== Palawan Installation Started: %s ===", start.Format("2006-01-02 15:04:05")))
	fmt.Printf("%sStarting Palawan installation...%s\n", palette.green, palette.reset)
	return start
}

func stopInstallLog(logFile *os.File, start time.Time, palette colors) {
	end := time.Now()
	logLine(logFile, fmt.Sprintf("=== Palawan Installation Completed: %s ===", end.Format("2006-01-02 15:04:05")))
	duration := int(end.Sub(start).Seconds())
	fmt.Fprintln(logFile, "")
	fmt.Fprintln(logFile, "[Installation Time Summary]")
	fmt.Fprintf(logFile, "Palawan:     %dm %ds\n", duration/60, duration%60)
	fmt.Fprintln(logFile, "=================================")
	fmt.Printf("%sInstallation complete.%s\n", palette.green, palette.reset)
}

func printSection(text string) {
	width := len(text)
	border := strings.Repeat("-", width+2)
	fmt.Printf("+%s+\n", border)
	fmt.Printf("| %s |\n", text)
	fmt.Printf("+%s+\n", border)
}

func printProgress(step int, total int, label string) {
	barWidth := 30
	filled := (barWidth * step) / total
	bar := strings.Repeat("#", filled) + strings.Repeat("-", barWidth-filled)
	fmt.Printf("[%s] %d/%d %s\n", bar, step, total, label)
}

func streamOutput(reader io.Reader, logFile *os.File) error {
	buffer := bufio.NewReader(reader)
	for {
		line, err := buffer.ReadString('\n')
		if line != "" {
			fmt.Print(line)
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

func runCommand(command []string, logFile *os.File, palette colors, description string) int {
	logLine(logFile, fmt.Sprintf("Starting: %s", description))
	cmd := exec.Command(command[0], command[1:]...)
	cmd.Env = os.Environ()
	output, err := cmd.StdoutPipe()
	if err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (pipe error: %v)", description, err))
		fmt.Printf("%sFailed: %s (pipe error)%s\n", palette.red, description, palette.reset)
		return 1
	}
	cmd.Stderr = cmd.Stdout

	if err := cmd.Start(); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (start error: %v)", description, err))
		fmt.Printf("%sFailed: %s (start error)%s\n", palette.red, description, palette.reset)
		return 1
	}

	if err := streamOutput(output, logFile); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (stream error: %v)", description, err))
		fmt.Printf("%sFailed: %s (stream error)%s\n", palette.red, description, palette.reset)
		_ = cmd.Wait()
		return 1
	}

	if err := cmd.Wait(); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (exit error: %v)", description, err))
		fmt.Printf("%sFailed: %s (exit error)%s\n", palette.red, description, palette.reset)
		return 1
	}

	logLine(logFile, fmt.Sprintf("Completed: %s", description))
	return 0
}

func runScript(scriptPath string, installDir string, logFile *os.File, palette colors) int {
	logLine(logFile, fmt.Sprintf("Starting: %s", scriptPath))
	command := fmt.Sprintf("source '%s/helpers/all.sh'; source '%s'", installDir, scriptPath)
	cmd := exec.Command("bash", "-c", command)
	cmd.Env = os.Environ()
	output, err := cmd.StdoutPipe()
	if err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (pipe error: %v)", scriptPath, err))
		fmt.Printf("%sFailed: %s (pipe error)%s\n", palette.red, scriptPath, palette.reset)
		return 1
	}
	cmd.Stderr = cmd.Stdout

	if err := cmd.Start(); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (start error: %v)", scriptPath, err))
		fmt.Printf("%sFailed: %s (start error)%s\n", palette.red, scriptPath, palette.reset)
		return 1
	}

	if err := streamOutput(output, logFile); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (stream error: %v)", scriptPath, err))
		fmt.Printf("%sFailed: %s (stream error)%s\n", palette.red, scriptPath, palette.reset)
		_ = cmd.Wait()
		return 1
	}

	if err := cmd.Wait(); err != nil {
		logLine(logFile, fmt.Sprintf("Failed: %s (exit error: %v)", scriptPath, err))
		fmt.Printf("%sFailed: %s (exit error)%s\n", palette.red, scriptPath, palette.reset)
		return 1
	}

	logLine(logFile, fmt.Sprintf("Completed: %s", scriptPath))
	return 0
}

func installBasePackages(installDir string, logFile *os.File, palette colors) int {
	printSection("=== Installing base packages ===")
	packagesPath := filepath.Join(installDir, "palawan-base.packages")
	content, err := os.ReadFile(packagesPath)
	if err != nil {
		logLine(logFile, fmt.Sprintf("Missing packages file: %s", packagesPath))
		return 1
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
		return 0
	}

	command := append([]string{"sudo", "pacman", "-S", "--noconfirm", "--needed"}, packages...)
	return runCommand(command, logFile, palette, "install base packages")
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

	scripts := []string{
		filepath.Join(installDir, "packages/drivers/amd-gpu-cpu.sh"),
		filepath.Join(installDir, "packages/yay.sh"),
		filepath.Join(installDir, "packages/hyprland.sh"),
		filepath.Join(installDir, "packages/nodejs.sh"),
		filepath.Join(installDir, "packages/terminal.sh"),
		filepath.Join(installDir, "packages/ai-cli/codex-cli.sh"),
		filepath.Join(installDir, "packages/ai-cli/gemini-cli.sh"),
		filepath.Join(installDir, "packages/ai-cli/claude-cli.sh"),
		filepath.Join(installDir, "packages/fonts.sh"),
		filepath.Join(installDir, "packages/web-browser.sh"),
		filepath.Join(installDir, "packages/editors.sh"),
		filepath.Join(installDir, "greeter/sddm/sddm.sh"),
		filepath.Join(installDir, "post-install/gnome-theme.sh"),
	}

	palette := initColors()
	start := startInstallLog(logFile, palette)

	totalSteps := 1 + len(scripts)
	step := 1

	printProgress(step, totalSteps, "base packages")
	if code := installBasePackages(installDir, logFile, palette); code != 0 {
		stopInstallLog(logFile, start, palette)
		os.Exit(code)
	}
	step++

	for _, script := range scripts {
		label := strings.TrimPrefix(script, installDir+string(os.PathSeparator))
		printProgress(step, totalSteps, label)
		if _, err := os.Stat(script); err != nil {
			logLine(logFile, fmt.Sprintf("Skipped missing: %s", script))
			step++
			continue
		}
		if code := runScript(script, installDir, logFile, palette); code != 0 {
			stopInstallLog(logFile, start, palette)
			os.Exit(code)
		}
		step++
	}

	stopInstallLog(logFile, start, palette)
}
