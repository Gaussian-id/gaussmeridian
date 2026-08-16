#!/usr/bin/env bash

set -euo pipefail

###############################################################################
# GaussMeridian process manager
#
# Controls:
#   - Backend (Rust HTTP API: `cargo run --bin gaussmeridian --release`)
#   - Frontend (Deno Fresh WebUI: `deno task start`)
#
# Usage:
#   ./gaussmeridian-manage.sh start [backend|frontend|all]
#   ./gaussmeridian-manage.sh stop [backend|frontend|all]
#   ./gaussmeridian-manage.sh restart [backend|frontend|all]
#   ./gaussmeridian-manage.sh status [backend|frontend|all]
#   ./gaussmeridian-manage.sh logs [backend|frontend|all]
#
# Notes:
#   - Designed for macOS (uses `pgrep`, `kill`, `lsof`).
#   - Uses only Deno (no npm) for the WebUI.
###############################################################################

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Repository layout:
#   /Users/.../GaussMeridian              -> SCRIPT_DIR
#   /Users/.../GaussMeridian/gaussmeridian -> Rust workspace
REPO_ROOT="${SCRIPT_DIR}"
BACKEND_ROOT="${REPO_ROOT}/gaussmeridian"
FRONTEND_ROOT="${BACKEND_ROOT}/services/webui"

RUNTIME_DIR="${REPO_ROOT}/.runtime"
PID_DIR="${RUNTIME_DIR}/pids"
LOG_DIR="${RUNTIME_DIR}/logs"

BACKEND_NAME="gaussmeridian-backend"
FRONTEND_NAME="gaussmeridian-webui"

BACKEND_PID_FILE="${PID_DIR}/${BACKEND_NAME}.pid"
FRONTEND_PID_FILE="${PID_DIR}/${FRONTEND_NAME}.pid"

BACKEND_LOG_FILE="${LOG_DIR}/${BACKEND_NAME}.log"
FRONTEND_LOG_FILE="${LOG_DIR}/${FRONTEND_NAME}.log"

DEFAULT_TARGET="all"

mkdir -p "${PID_DIR}" "${LOG_DIR}"

###############################################################################
# Environment loading
# - We keep DB and other sensitive config in .env-style files rather than
#   hard-coding them in this script.
# - This ensures GAUSSMERIDIAN_DB_* variables are available when the backend
#   boots, so it can establish a SurrealDB connection and run migrations.
###############################################################################

load_env() {
  # Allow callers to opt out if they really want a clean environment
  if [[ "${GAUSSMERIDIAN_SKIP_ENV_LOAD:-0}" == "1" ]]; then
    return
  fi

  # Export all variables while sourcing, then restore -a
  local old_set_a="0"
  if set -o | grep -q "allexport[[:space:]]*on"; then
    old_set_a="1"
  fi

  set -a

  # Repo-level .env (preferred for shared config like DB)
  if [[ -f "${REPO_ROOT}/.env" ]]; then
    # shellcheck disable=SC1090
    . "${REPO_ROOT}/.env"
  fi

  # Backend-specific .env overrides
  if [[ -f "${BACKEND_ROOT}/.env" ]]; then
    # shellcheck disable=SC1090
    . "${BACKEND_ROOT}/.env"
  fi

  # Frontend-specific .env overrides
  if [[ -f "${FRONTEND_ROOT}/.env" ]]; then
    # shellcheck disable=SC1090
    . "${FRONTEND_ROOT}/.env"
  fi

  if [[ "${old_set_a}" == "0" ]]; then
    set +a
  fi
}

usage() {
  cat <<EOF
GaussMeridian process manager

Usage:
  $(basename "$0") start   [backend|frontend|all]
  $(basename "$0") stop    [backend|frontend|all]
  $(basename "$0") restart [backend|frontend|all]
  $(basename "$0") status  [backend|frontend|all]
  $(basename "$0") logs    [backend|frontend|all]
  $(basename "$0") help

Examples:
  $(basename "$0") start
  $(basename "$0") start backend
  $(basename "$0") restart all
  $(basename "$0") logs frontend
EOF
}

log() {
  local level="$1"; shift
  local ts
  ts="$(date '+%Y-%m-%d %H:%M:%S')"
  echo "[$ts] [$level] $*"
}

error() {
  log "ERROR" "$*" >&2
}

warn() {
  log "WARN" "$*" >&2
}

info() {
  log "INFO" "$*"
}

debug() {
  if [[ "${DEBUG:-0}" == "1" ]]; then
    log "DEBUG" "$*"
  fi
}

pid_is_running() {
  local pid="$1"
  if [[ -z "${pid}" ]]; then
    return 1
  fi
  if kill -0 "${pid}" 2>/dev/null; then
    return 0
  fi
  return 1
}

read_pid() {
  local file="$1"
  if [[ -f "${file}" ]]; then
    # shellcheck disable=SC2002
    cat "${file}" 2>/dev/null || true
  else
    echo ""
  fi
}

write_pid() {
  local pid="$1"
  local file="$2"
  echo "${pid}" > "${file}"
}

remove_pid_file() {
  local file="$1"
  if [[ -f "${file}" ]]; then
    rm -f "${file}"
  fi
}

kill_process() {
  local name="$1"
  local pid_file="$2"

  local pid
  pid="$(read_pid "${pid_file}")"

  if [[ -z "${pid}" ]]; then
    info "${name} is not running (no PID file)."
    return 0
  fi

  if ! pid_is_running "${pid}"; then
    info "${name} is not running (stale PID ${pid}). Cleaning up."
    remove_pid_file "${pid_file}"
    return 0
  fi

  info "Stopping ${name} (PID ${pid})..."
  kill "${pid}" 2>/dev/null || true

  local waited=0
  local timeout=15

  while pid_is_running "${pid}" && [[ "${waited}" -lt "${timeout}" ]]; do
    sleep 1
    waited=$((waited + 1))
  done

  if pid_is_running "${pid}"; then
    warn "${name} (PID ${pid}) did not exit gracefully; sending SIGKILL."
    kill -9 "${pid}" 2>/dev/null || true
  fi

  remove_pid_file "${pid_file}"
  info "${name} stopped."
}

start_backend() {
  if [[ ! -d "${BACKEND_ROOT}" ]]; then
    error "Backend directory not found: ${BACKEND_ROOT}"
    exit 1
  fi

  local existing_pid
  existing_pid="$(read_pid "${BACKEND_PID_FILE}")"
  if pid_is_running "${existing_pid}"; then
    info "Backend already running with PID ${existing_pid}."
    return 0
  elif [[ -n "${existing_pid}" ]]; then
    warn "Cleaning up stale backend PID file (PID ${existing_pid})."
    remove_pid_file "${BACKEND_PID_FILE}"
  fi

  info "Starting backend (Rust API server)..."
  (
    cd "${BACKEND_ROOT}"
    # Allow caller to override RUST_LOG; default to debug for dev
    export RUST_LOG="${RUST_LOG:-debug}"
    # Run in release mode as per SETUP.md
    cargo run --bin gaussmeridian --release >> "${BACKEND_LOG_FILE}" 2>&1 &
    echo $! > "${BACKEND_PID_FILE}"
  )

  local pid
  pid="$(read_pid "${BACKEND_PID_FILE}")"
  if [[ -z "${pid}" ]] || ! pid_is_running "${pid}"; then
    error "Failed to start backend. See log: ${BACKEND_LOG_FILE}"
    exit 1
  fi

  info "Backend started with PID ${pid}. Logs: ${BACKEND_LOG_FILE}"
}

start_frontend() {
  if [[ ! -d "${FRONTEND_ROOT}" ]]; then
    error "Frontend directory not found: ${FRONTEND_ROOT}"
    exit 1
  fi

  if ! command -v deno >/dev/null 2>&1; then
    error "Deno is not installed or not in PATH. Please install Deno 2.0+."
    exit 1
  fi

  # Prefer explicit Deno binary if provided, otherwise fall back to system 'deno'
  local deno_bin="${DENO_BIN:-$HOME/.deno/bin/deno}"
  if ! command -v "${deno_bin}" >/dev/null 2>&1; then
    deno_bin="deno"
  fi

  local existing_pid
  existing_pid="$(read_pid "${FRONTEND_PID_FILE}")"
  if pid_is_running "${existing_pid}"; then
    info "Frontend already running with PID ${existing_pid}."
    return 0
  elif [[ -n "${existing_pid}" ]]; then
    warn "Cleaning up stale frontend PID file (PID ${existing_pid})."
    remove_pid_file "${FRONTEND_PID_FILE}"
  fi

  info "Starting frontend (Deno Fresh WebUI)..."
  (
    cd "${FRONTEND_ROOT}"
    # Use Deno task defined in deno.json; no npm usage.
    "${deno_bin}" task start >> "${FRONTEND_LOG_FILE}" 2>&1 &
    echo $! > "${FRONTEND_PID_FILE}"
  )

  local pid
  pid="$(read_pid "${FRONTEND_PID_FILE}")"
  if [[ -z "${pid}" ]] || ! pid_is_running "${pid}"; then
    error "Failed to start frontend. See log: ${FRONTEND_LOG_FILE}"
    exit 1
  fi

  info "Frontend started with PID ${pid}. Logs: ${FRONTEND_LOG_FILE}"
}

status_backend() {
  local pid
  pid="$(read_pid "${BACKEND_PID_FILE}")"
  if [[ -n "${pid}" ]] && pid_is_running "${pid}"; then
    echo "Backend: RUNNING (PID ${pid})"
  else
    echo "Backend: STOPPED"
  fi
}

status_frontend() {
  local pid
  pid="$(read_pid "${FRONTEND_PID_FILE}")"
  if [[ -n "${pid}" ]] && pid_is_running "${pid}"; then
    echo "Frontend: RUNNING (PID ${pid})"
  else
    echo "Frontend: STOPPED"
  fi
}

tail_logs() {
  local component="$1"

  case "${component}" in
    backend)
      touch "${BACKEND_LOG_FILE}"
      info "Tailing backend logs (Ctrl+C to stop)..."
      tail -f "${BACKEND_LOG_FILE}"
      ;;
    frontend)
      touch "${FRONTEND_LOG_FILE}"
      info "Tailing frontend logs (Ctrl+C to stop)..."
      tail -f "${FRONTEND_LOG_FILE}"
      ;;
    all)
      touch "${BACKEND_LOG_FILE}" "${FRONTEND_LOG_FILE}"
      info "Tailing backend and frontend logs (Ctrl+C to stop)..."
      tail -f "${BACKEND_LOG_FILE}" "${FRONTEND_LOG_FILE}"
      ;;
    *)
      error "Unknown component for logs: ${component}"
      exit 1
      ;;
  esac
}

normalize_target() {
  local t="${1:-${DEFAULT_TARGET}}"
  case "${t}" in
    backend|front|frontend|webui)
      echo "backend_or_frontend:${t}"
      ;;
    all|"")
      echo "all"
      ;;
    *)
      error "Unknown target: ${t}"
      exit 1
      ;;
  esac
}

do_start() {
  local target="${1:-${DEFAULT_TARGET}}"
  case "${target}" in
    backend)
      start_backend
      ;;
    frontend|webui|front)
      start_frontend
      ;;
    all)
      start_backend
      start_frontend
      ;;
    *)
      error "Unknown target for start: ${target}"
      exit 1
      ;;
  esac
}

do_stop() {
  local target="${1:-${DEFAULT_TARGET}}"
  case "${target}" in
    backend)
      kill_process "backend" "${BACKEND_PID_FILE}"
      ;;
    frontend|webui|front)
      kill_process "frontend" "${FRONTEND_PID_FILE}"
      ;;
    all)
      kill_process "frontend" "${FRONTEND_PID_FILE}"
      kill_process "backend" "${BACKEND_PID_FILE}"
      ;;
    *)
      error "Unknown target for stop: ${target}"
      exit 1
      ;;
  esac
}

do_restart() {
  local target="${1:-${DEFAULT_TARGET}}"
  do_stop "${target}"
  do_start "${target}"
}

do_status() {
  local target="${1:-${DEFAULT_TARGET}}"
  case "${target}" in
    backend)
      status_backend
      ;;
    frontend|webui|front)
      status_frontend
      ;;
    all)
      status_backend
      status_frontend
      ;;
    *)
      error "Unknown target for status: ${target}"
      exit 1
      ;;
  esac
}

do_logs() {
  local target="${1:-${DEFAULT_TARGET}}"
  case "${target}" in
    backend)
      tail_logs backend
      ;;
    frontend|webui|front)
      tail_logs frontend
      ;;
    all)
      tail_logs all
      ;;
    *)
      error "Unknown target for logs: ${target}"
      exit 1
      ;;
  esac
}

main() {
  if [[ $# -lt 1 ]]; then
    usage
    exit 1
  fi

  local cmd="$1"
  shift || true

  local target="${1:-${DEFAULT_TARGET}}"

  case "${cmd}" in
    start)
      do_start "${target}"
      ;;
    stop)
      do_stop "${target}"
      ;;
    restart)
      do_restart "${target}"
      ;;
    status)
      do_status "${target}"
      ;;
    logs)
      do_logs "${target}"
      ;;
    help|-h|--help)
      usage
      ;;
    *)
      error "Unknown command: ${cmd}"
      usage
      exit 1
      ;;
  esac
}

main "$@"


