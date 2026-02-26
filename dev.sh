#!/usr/bin/env bash
# Root Editorial - Dev Dashboard
# Usage: ./dev.sh              Start services + live dashboard
#        ./dev.sh status       One-shot status check
#        ./dev.sh start        Start services (no dashboard)
#        ./dev.sh stop         Stop all services
#        ./dev.sh restart      Restart all services
#        ./dev.sh logs [svc]   Follow logs (all or specific service)
set -uo pipefail

# ── Project root ─────────────────────────────────────────────────────
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT" || exit 1

# ── Container names ──────────────────────────────────────────────────
C_POSTGRES="mndigitalaid_postgres"
C_REDIS="mndigitalaid_redis"
C_RESTATE="mndigitalaid_restate"
C_SERVER="mndigitalaid_server"
C_ADMIN="mndigitalaid_admin_app"
C_WEBAPP="mndigitalaid_web_app"

# ── Ports ────────────────────────────────────────────────────────────
P_POSTGRES=5432
P_REDIS=6379
P_RESTATE=8180
P_SERVER=9080
P_ADMIN=3000
P_WEBAPP=3001

# ── Colors ───────────────────────────────────────────────────────────
if [[ -t 1 ]]; then
  GREEN='\033[0;32m'
  RED='\033[0;31m'
  YELLOW='\033[0;33m'
  DIM='\033[2m'
  BOLD='\033[1m'
  RESET='\033[0m'
else
  GREEN='' RED='' YELLOW='' DIM='' BOLD='' RESET=''
fi

# ── Health checks ────────────────────────────────────────────────────

check_docker_engine() {
  docker info >/dev/null 2>&1
}

# Returns: running, healthy, unhealthy, starting, stopped, unknown
container_state() {
  local name="$1"
  local state health

  state=$(docker inspect --format='{{.State.Status}}' "$name" 2>/dev/null) || {
    echo "stopped"
    return
  }

  if [[ "$state" != "running" ]]; then
    echo "$state"
    return
  fi

  # Check if container has a health check defined
  health=$(docker inspect --format='{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$name" 2>/dev/null) || {
    echo "running"
    return
  }

  case "$health" in
    healthy)   echo "healthy" ;;
    starting)  echo "starting" ;;
    unhealthy) echo "unhealthy" ;;
    none)      echo "running" ;;
    *)         echo "running" ;;
  esac
}

# Check if a port has something listening (fallback for local processes)
is_port_listening() {
  local port="$1"
  lsof -i ":$port" -sTCP:LISTEN >/dev/null 2>&1
}

# Get a service status suitable for display
# Returns: ok, starting, fail, stopped
# Also sets SERVICE_HINT with recovery text when relevant
SERVICE_HINT=""
get_status() {
  local container="$1"
  local port="$2"
  SERVICE_HINT=""

  local state
  state=$(container_state "$container")

  case "$state" in
    healthy)
      echo "ok"
      ;;
    running)
      # Running but no health check, or health check not defined — check port
      if is_port_listening "$port"; then
        echo "ok"
      else
        echo "starting"
        SERVICE_HINT="Container running, waiting for port $port..."
      fi
      ;;
    starting)
      echo "starting"
      SERVICE_HINT="Health check pending..."
      ;;
    unhealthy)
      echo "fail"
      SERVICE_HINT="Container unhealthy. Run: docker compose logs ${container##mndigitalaid_}"
      ;;
    stopped|exited)
      # Check if something else is on the port (local process)
      if is_port_listening "$port"; then
        echo "ok"
        SERVICE_HINT="(local process)"
      else
        echo "stopped"
      fi
      ;;
    *)
      echo "stopped"
      ;;
  esac
}

# ── Service management ───────────────────────────────────────────────

start_services() {
  echo "Starting services..."
  docker compose up -d 2>&1
  echo ""
  echo "Services started. Backend may take 1-2 min to compile on first run."
}

stop_services() {
  echo "Stopping services..."
  docker compose down 2>&1
  echo "Done."
}

restart_services() {
  echo "Restarting services..."
  docker compose down 2>&1
  docker compose up -d 2>&1
  echo ""
  echo "Services restarted."
}

rebuild_server() {
  echo "Rebuilding server..."
  docker compose up -d --build server 2>&1
  echo ""
  echo "Server rebuild triggered. Watch the dashboard for status."
}

# ── Display ──────────────────────────────────────────────────────────

# Status indicator: OK, FAIL, .., --
status_label() {
  local status="$1"
  case "$status" in
    ok)       printf "${GREEN}${BOLD} OK  ${RESET}" ;;
    starting) printf "${YELLOW}${BOLD} ..  ${RESET}" ;;
    fail)     printf "${RED}${BOLD}FAIL ${RESET}" ;;
    stopped)  printf "${DIM} --  ${RESET}" ;;
  esac
}

# Render a single service line
render_service() {
  local status="$1"
  local label="$2"
  local port="$3"
  local extra="${4:-}"

  status_label "$status"
  printf " %-24s :%-5s" "$label" "$port"

  if [[ -n "$extra" ]]; then
    printf "  ${DIM}%s${RESET}" "$extra"
  fi
  echo ""

  # Show hint line for failures
  if [[ -n "$SERVICE_HINT" && ("$status" == "fail" || "$status" == "starting") ]]; then
    printf "       ${DIM}%s${RESET}\n" "$SERVICE_HINT"
  fi
}

# Render a service with a clickable URL instead of port
render_service_url() {
  local status="$1"
  local label="$2"
  local port="$3"
  local extra="${4:-}"

  status_label "$status"

  if [[ "$status" == "ok" ]]; then
    printf " %-20s --> ${BOLD}http://localhost:%s${RESET}" "$label" "$port"
  else
    printf " %-24s :%-5s" "$label" "$port"
  fi

  if [[ -n "$extra" ]]; then
    printf "  ${DIM}%s${RESET}" "$extra"
  fi
  echo ""

  if [[ -n "$SERVICE_HINT" && ("$status" == "fail" || "$status" == "starting") ]]; then
    printf "       ${DIM}%s${RESET}\n" "$SERVICE_HINT"
  fi
}

render_dashboard() {
  local clear_screen="${1:-true}"

  # Gather all statuses
  local s_pg s_redis s_restate s_server s_admin s_webapp
  local h_pg h_redis h_restate h_server h_admin h_webapp

  s_pg=$(get_status "$C_POSTGRES" "$P_POSTGRES"); h_pg="$SERVICE_HINT"
  s_redis=$(get_status "$C_REDIS" "$P_REDIS"); h_redis="$SERVICE_HINT"
  s_restate=$(get_status "$C_RESTATE" "$P_RESTATE"); h_restate="$SERVICE_HINT"
  s_server=$(get_status "$C_SERVER" "$P_SERVER"); h_server="$SERVICE_HINT"
  s_admin=$(get_status "$C_ADMIN" "$P_ADMIN"); h_admin="$SERVICE_HINT"
  s_webapp=$(get_status "$C_WEBAPP" "$P_WEBAPP"); h_webapp="$SERVICE_HINT"

  if [[ "$clear_screen" == "true" ]]; then
    printf '\033[2J\033[H'
  fi

  echo ""
  printf "  ${BOLD}Root Editorial Dev${RESET}\n"
  echo "  -------------------------------------------"
  echo ""
  printf "  ${BOLD}Infrastructure${RESET}\n"

  SERVICE_HINT="$h_pg";      render_service "$s_pg"      "PostgreSQL"    "$P_POSTGRES"
  SERVICE_HINT="$h_redis";   render_service "$s_redis"   "Redis"         "$P_REDIS"
  SERVICE_HINT="$h_restate"; render_service "$s_restate" "Restate"       "$P_RESTATE"

  echo ""
  printf "  ${BOLD}Backend${RESET}\n"
  SERVICE_HINT="$h_server";  render_service "$s_server"  "Rust Server"   "$P_SERVER"

  echo ""
  printf "  ${BOLD}Frontend${RESET}\n"
  SERVICE_HINT="$h_admin";   render_service_url "$s_admin"  "Admin App (CMS)" "$P_ADMIN"
  SERVICE_HINT="$h_webapp";  render_service_url "$s_webapp" "Web App"         "$P_WEBAPP"

  echo ""
  echo "  -------------------------------------------"

  if [[ "$clear_screen" == "true" ]]; then
    printf "  ${DIM}[s]${RESET} start  ${DIM}[r]${RESET} restart  ${DIM}[b]${RESET} rebuild server  ${DIM}[l]${RESET} logs  ${DIM}[q]${RESET} quit\n"
    echo ""
    printf "  ${DIM}Updated %s${RESET}\n" "$(date +%H:%M:%S)"
  fi
}

# ── Log viewing ──────────────────────────────────────────────────────

show_logs() {
  local service="${1:-}"
  echo ""
  echo "Following logs... (Ctrl+C to return to dashboard)"
  echo ""
  if [[ -n "$service" ]]; then
    docker compose logs -f --tail 50 "$service"
  else
    docker compose logs -f --tail 30
  fi
}

# ── Interactive dashboard loop ───────────────────────────────────────

dashboard_loop() {
  # Auto-start if nothing is running
  local pg_state
  pg_state=$(container_state "$C_POSTGRES")
  if [[ "$pg_state" == "stopped" ]]; then
    start_services
    echo ""
    echo "Launching dashboard..."
    sleep 2
  fi

  # Main loop
  while true; do
    render_dashboard

    # Wait up to 3 seconds for a keypress
    local key=""
    read -rsn1 -t 3 key 2>/dev/null || true

    case "$key" in
      s|S)
        printf '\033[2J\033[H'
        start_services
        sleep 2
        ;;
      r|R)
        printf '\033[2J\033[H'
        restart_services
        sleep 2
        ;;
      b|B)
        printf '\033[2J\033[H'
        rebuild_server
        sleep 2
        ;;
      l|L)
        show_logs
        # After Ctrl+C from logs, resume dashboard
        ;;
      q|Q)
        echo ""
        exit 0
        ;;
    esac
  done
}

# ── Entry point ──────────────────────────────────────────────────────

# Check Docker first
if ! check_docker_engine; then
  echo ""
  echo "Docker is not running."
  echo "Start Docker Desktop and try again."
  echo ""
  exit 1
fi

case "${1:-}" in
  start)
    start_services
    ;;
  stop)
    stop_services
    ;;
  restart)
    restart_services
    ;;
  status)
    render_dashboard false
    ;;
  logs)
    show_logs "${2:-}"
    ;;
  help|--help|-h)
    echo "Root Editorial Dev Dashboard"
    echo ""
    echo "Usage: ./dev.sh [command]"
    echo ""
    echo "Commands:"
    echo "  (none)     Start services + live dashboard"
    echo "  status     One-shot status check"
    echo "  start      Start all services"
    echo "  stop       Stop all services"
    echo "  restart    Restart all services"
    echo "  logs [svc] Follow logs (all or specific service)"
    echo "  help       Show this help"
    echo ""
    echo "Dashboard shortcuts:"
    echo "  [s] start all   [r] restart   [b] rebuild server"
    echo "  [l] logs        [q] quit"
    ;;
  *)
    dashboard_loop
    ;;
esac
