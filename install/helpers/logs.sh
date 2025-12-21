log_line() {
  local message="$1"
  printf '[%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$message" >>"$PALAWAN_INSTALL_LOG_FILE"
}

start_install_log() {
  mkdir -p "$(dirname "$PALAWAN_INSTALL_LOG_FILE")"
  : >"$PALAWAN_INSTALL_LOG_FILE"

  export PALAWAN_START_TIME
  PALAWAN_START_TIME=$(date '+%Y-%m-%d %H:%M:%S')

  log_line "=== Palawan Installation Started: $PALAWAN_START_TIME ==="
}

stop_install_log() {
  if [[ -n ${PALAWAN_INSTALL_LOG_FILE:-} ]]; then
    local palawan_end_time
    palawan_end_time=$(date '+%Y-%m-%d %H:%M:%S')

    log_line "=== Palawan Installation Completed: $palawan_end_time ==="
    echo "" >>"$PALAWAN_INSTALL_LOG_FILE"
    log_line "=== Installation Time Summary ==="

    if [ -n "$PALAWAN_START_TIME" ]; then
      local palawan_start_epoch palawan_end_epoch palawan_duration
      palawan_start_epoch=$(date -d "$PALAWAN_START_TIME" +%s)
      palawan_end_epoch=$(date -d "$palawan_end_time" +%s)
      palawan_duration=$((palawan_end_epoch - palawan_start_epoch))

      printf 'Palawan:     %dm %ds\n' $((palawan_duration / 60)) $((palawan_duration % 60)) >>"$PALAWAN_INSTALL_LOG_FILE"
    fi

    log_line "================================="
    log_line "Rebooting system..."
  fi
}

run_logged() {
  local script="$1"

  export CURRENT_SCRIPT="$script"

  log_line "Starting: $script"

  bash -c "source '$PALAWAN_INSTALL/helpers/all.sh'; source '$script'" 2>&1 | tee -a "$PALAWAN_INSTALL_LOG_FILE"
  local exit_code=${PIPESTATUS[0]}

  if [ $exit_code -eq 0 ]; then
    log_line "Completed: $script"
    unset CURRENT_SCRIPT
  else
    log_line "Failed: $script (exit code: $exit_code)"
  fi

  return $exit_code
}
